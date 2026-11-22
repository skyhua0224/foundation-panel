//! User-session owner for Sunshine's temporary touch-keyboard preference.
//!
//! The tray monitor supplies authenticated local state. Registry I/O and durable
//! journal writes run on one worker, never on the tray or WebView thread.

mod notification;
mod transaction;

use notification::WnfNotification;

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::windows::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::JoinHandle;
use std::time::Duration;

use transaction::{Journal, MAX_JOURNAL_BYTES, MAX_VALUE_BYTES, RawValue, Registry, Transaction};
use windows::Win32::Foundation::{
    ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA, ERROR_SUCCESS, WIN32_ERROR,
};
use windows::Win32::System::Registry::{
    HKEY, REG_VALUE_TYPE, RegDeleteValueW, RegFlushKey, RegQueryValueExW, RegSetValueExW,
};
use windows::core::PCWSTR;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE};

const REGISTRY_PATH: &str = r"Software\Microsoft\TabletTip\1.7";
const CAPABILITY: &str = "touch-keyboard-v1";
const RETRY_INITIAL: Duration = Duration::from_secs(3);
const RETRY_MAX: Duration = Duration::from_secs(30);

#[link(name = "kernel32")]
unsafe extern "system" {
    fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
}

fn current_session_id() -> io::Result<u32> {
    let mut session_id = 0;
    if unsafe { ProcessIdToSessionId(std::process::id(), &mut session_id) } == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(session_id)
    }
}

fn requested_mode(
    state: Option<&crate::sunshine::TrayState>,
    own_session: Option<u32>,
) -> Option<bool> {
    let state = state?;
    if !state.capabilities.iter().any(|value| value == CAPABILITY) {
        return None;
    }
    let target = state.touch_keyboard?;
    (own_session == Some(target.session_id)).then_some(target.enabled)
}

enum Command {
    Update(Option<bool>),
    Stop,
}

struct Worker {
    sender: mpsc::Sender<Command>,
    thread: JoinHandle<()>,
    session_id: Option<u32>,
    stopping: Arc<AtomicBool>,
    last_enqueued: Option<Option<bool>>,
    last_mismatch: Option<u32>,
}

static WORKER: Mutex<Option<Worker>> = Mutex::new(None);

/// The setup-local guard stops the worker on setup failure as well as on exit.
pub(crate) struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        stop();
    }
}

pub(crate) fn start() -> Guard {
    let mut owner = WORKER.lock().unwrap_or_else(|error| error.into_inner());
    if owner.is_some() {
        return Guard;
    }
    let session_id = match current_session_id() {
        Ok(id) => Some(id),
        Err(error) => {
            log::warn!("touch keyboard session ID is unavailable: {error}");
            None
        }
    };
    let (sender, receiver) = mpsc::channel();
    let stopping = Arc::new(AtomicBool::new(false));
    let worker_stopping = stopping.clone();
    match std::thread::Builder::new()
        .name("touch-keyboard".into())
        .spawn(move || {
            let journal = match FileJournal::new() {
                Ok(journal) => journal,
                Err(error) => {
                    log::warn!("touch keyboard user data path is unavailable: {error}");
                    return;
                }
            };
            let mut transaction =
                Transaction::new(UserRegistry::default(), journal, WnfNotification::default());
            let mut retry = match run_transaction(&mut transaction, |tx| tx.recover_on_start()) {
                Ok(()) => None,
                Err(error) => {
                    log::warn!("touch keyboard startup recovery deferred: {error}");
                    Some(RETRY_INITIAL)
                }
            };
            let mut mode = None;
            let mut backoff = RETRY_INITIAL;
            loop {
                let command = if let Some(delay) = retry.take() {
                    match receiver.recv_timeout(delay) {
                        Ok(command) => Some(command),
                        Err(mpsc::RecvTimeoutError::Timeout) => None,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                } else {
                    match receiver.recv() {
                        Ok(command) => Some(command),
                        Err(_) => break,
                    }
                };
                // Shutdown takes precedence over queued states. Other updates
                // stay ordered so a disconnect always restores before a later
                // reconnect can apply a new override.
                if worker_stopping.load(Ordering::Acquire) {
                    break;
                }
                match command {
                    Some(Command::Update(next)) => {
                        if mode != next {
                            backoff = RETRY_INITIAL;
                        }
                        mode = next;
                    }
                    Some(Command::Stop) => break,
                    None => {}
                }
                match run_transaction(&mut transaction, |tx| {
                    tx.reconcile(mode, session_id.unwrap_or_default())
                }) {
                    Ok(()) => {
                        let description = match mode {
                            Some(true) => "enabled",
                            Some(false) => "disabled",
                            None => "restored",
                        };
                        log::info!("touch keyboard preference reconciled: {description}");
                        backoff = RETRY_INITIAL;
                    }
                    Err(error) => {
                        log::warn!("touch keyboard state could not be reconciled: {error}");
                        retry = Some(backoff);
                        backoff = (backoff * 2).min(RETRY_MAX);
                    }
                }
            }
            if let Err(error) = run_transaction(&mut transaction, |tx| {
                tx.reconcile(None, session_id.unwrap_or_default())
            }) {
                log::warn!("touch keyboard exit recovery deferred: {error}");
            }
        }) {
        Ok(thread) => {
            *owner = Some(Worker {
                sender,
                thread,
                session_id,
                stopping,
                last_enqueued: None,
                last_mismatch: None,
            });
        }
        Err(error) => log::warn!("touch keyboard worker could not start: {error}"),
    }
    Guard
}

pub(crate) fn reconcile(state: Option<&crate::sunshine::TrayState>) {
    let mut owner = WORKER.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(worker) = owner.as_mut() {
        let mismatch = state
            .filter(|state| state.capabilities.iter().any(|value| value == CAPABILITY))
            .and_then(|state| state.touch_keyboard)
            .filter(|target| worker.session_id != Some(target.session_id))
            .map(|target| target.session_id);
        if mismatch != worker.last_mismatch {
            if let Some(session_id) = mismatch {
                log::warn!(
                    "touch keyboard preference skipped: GUI session {:?} differs from capture session {}",
                    worker.session_id,
                    session_id
                );
            }
            worker.last_mismatch = mismatch;
        }
        let mode = requested_mode(state, worker.session_id);
        if worker.last_enqueued != Some(mode) {
            if worker.sender.send(Command::Update(mode)).is_ok() {
                worker.last_enqueued = Some(mode);
            } else {
                log::warn!("touch keyboard worker is unavailable; the journal was retained");
            }
        }
    }
}

pub(crate) fn stop() {
    let worker = WORKER
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take();
    if let Some(worker) = worker {
        worker.stopping.store(true, Ordering::Release);
        let _ = worker.sender.send(Command::Stop);
        if worker.thread.join().is_err() {
            log::warn!("touch keyboard worker stopped unexpectedly; the journal was retained");
        }
    }
}

struct FileJournal {
    directory: PathBuf,
    path: PathBuf,
    lock: Option<File>,
}

fn run_transaction(
    transaction: &mut Transaction<UserRegistry, FileJournal, WnfNotification>,
    operation: impl FnOnce(
        &mut Transaction<UserRegistry, FileJournal, WnfNotification>,
    ) -> io::Result<()>,
) -> io::Result<()> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        operation(&mut *transaction)
    })) {
        Ok(result) => result,
        Err(_) => {
            // Release any held lock, then recover the durable journal before
            // another mode can be applied. This mirrors the clipboard agent's
            // ability to replace a failed worker without restarting the GUI.
            *transaction = Transaction::new(
                UserRegistry::default(),
                FileJournal::new()?,
                WnfNotification::default(),
            );
            Err(io::Error::other(
                "touch keyboard worker panicked; recovery will retry",
            ))
        }
    }
}

impl FileJournal {
    fn new() -> io::Result<Self> {
        let directory = crate::test_isolation::data_local_dir()
            .ok_or_else(|| io::Error::other("user local data directory is unavailable"))?
            .join("Sunshine GUI")
            .join("touch-keyboard");
        Ok(Self {
            path: directory.join("preference-v1.jsonl"),
            directory,
            lock: None,
        })
    }

    fn write_synced(file: &mut File, contents: &[u8]) -> io::Result<()> {
        file.write_all(contents)?;
        file.flush()?;
        file.sync_all()
    }

    fn create_with(
        &self,
        contents: &[u8],
        write: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        // Publish only a complete, durable snapshot. A failed write leaves the
        // destination absent and the temporary file is removed on drop.
        let mut temporary = tempfile::NamedTempFile::new_in(&self.directory)?;
        write(temporary.as_file_mut(), contents)?;
        temporary
            .persist_noclobber(&self.path)
            .map_err(|error| error.error)?;
        Ok(())
    }

    fn append_with(
        &self,
        contents: &[u8],
        write: impl FnOnce(&mut File, &[u8]) -> io::Result<()>,
    ) -> io::Result<()> {
        // The exclusive transaction lock serializes writers. Request normal
        // write access so sync_all can flush the Windows file handle.
        let mut file = OpenOptions::new().write(true).open(&self.path)?;
        let previous_len = file.seek(SeekFrom::End(0))?;
        if let Err(error) = write(&mut file, contents) {
            // The registry write follows a successful append, so this intent
            // is known not to have been applied. Remove only its partial tail.
            if let Err(cleanup_error) = file.set_len(previous_len).and_then(|()| file.sync_all()) {
                log::warn!("touch keyboard journal tail cleanup failed: {cleanup_error}");
            }
            return Err(error);
        }
        Ok(())
    }
}

impl Journal for FileJournal {
    fn lock(&mut self) -> io::Result<()> {
        if self.lock.is_some() {
            return Ok(());
        }
        fs::create_dir_all(&self.directory)?;
        self.lock = Some(
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .share_mode(0)
                .open(self.directory.join("transaction.lock"))?,
        );
        Ok(())
    }

    fn unlock(&mut self) {
        self.lock = None;
    }

    fn read(&mut self) -> io::Result<Option<Vec<u8>>> {
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        let mut bytes = Vec::new();
        file.take(MAX_JOURNAL_BYTES + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "touch keyboard journal is too large",
            ));
        }
        Ok(Some(bytes))
    }

    fn create(&mut self, contents: &[u8]) -> io::Result<()> {
        self.create_with(contents, Self::write_synced)
    }

    fn append(&mut self, contents: &[u8]) -> io::Result<()> {
        self.append_with(contents, Self::write_synced)
    }

    fn remove(&mut self) -> io::Result<()> {
        fs::remove_file(&self.path)
    }
}

struct UserRegistry {
    path: String,
}

impl Default for UserRegistry {
    fn default() -> Self {
        Self {
            path: REGISTRY_PATH.to_string(),
        }
    }
}

fn win32_result(status: WIN32_ERROR) -> io::Result<()> {
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status.0 as i32))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn native_key(key: &RegKey) -> HKEY {
    HKEY(key.raw_handle() as _)
}

fn open_user_key(path: &str, access: u32) -> io::Result<Option<RegKey>> {
    match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(path, access) {
        Ok(key) => Ok(Some(key)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

impl Registry for UserRegistry {
    fn read(&mut self, name: &str) -> io::Result<Option<RawValue>> {
        let Some(key) = open_user_key(&self.path, KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
        let name = wide(name);
        let mut kind = REG_VALUE_TYPE(0);
        let mut size = 0u32;
        let status = unsafe {
            RegQueryValueExW(
                native_key(&key),
                PCWSTR(name.as_ptr()),
                None,
                Some(&mut kind),
                None,
                Some(&mut size),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        win32_result(status)?;
        for _ in 0..4 {
            if size as usize > MAX_VALUE_BYTES {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "touch keyboard registry value is too large",
                ));
            }
            let mut bytes = vec![0; size as usize];
            let status = unsafe {
                RegQueryValueExW(
                    native_key(&key),
                    PCWSTR(name.as_ptr()),
                    None,
                    Some(&mut kind),
                    Some(bytes.as_mut_ptr()),
                    Some(&mut size),
                )
            };
            if status == ERROR_FILE_NOT_FOUND {
                return Ok(None);
            }
            if status == ERROR_MORE_DATA {
                continue;
            }
            win32_result(status)?;
            bytes.truncate(size as usize);
            return Ok(Some(RawValue {
                kind: kind.0,
                bytes,
            }));
        }
        Err(io::Error::other(
            "touch keyboard registry value changed repeatedly while reading",
        ))
    }

    fn set(&mut self, name: &str, value: &RawValue) -> io::Result<()> {
        let key = match open_user_key(&self.path, KEY_QUERY_VALUE | KEY_SET_VALUE)? {
            Some(key) => key,
            None => {
                RegKey::predef(HKEY_CURRENT_USER)
                    .create_subkey_with_flags(&self.path, KEY_QUERY_VALUE | KEY_SET_VALUE)?
                    .0
            }
        };
        let name = wide(name);
        win32_result(unsafe {
            RegSetValueExW(
                native_key(&key),
                PCWSTR(name.as_ptr()),
                None,
                REG_VALUE_TYPE(value.kind),
                Some(&value.bytes),
            )
        })
    }

    fn delete(&mut self, name: &str) -> io::Result<()> {
        let Some(key) = open_user_key(&self.path, KEY_SET_VALUE)? else {
            return Ok(());
        };
        let name = wide(name);
        let status = unsafe { RegDeleteValueW(native_key(&key), PCWSTR(name.as_ptr())) };
        if status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            win32_result(status)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let Some(key) = open_user_key(&self.path, KEY_QUERY_VALUE)? else {
            return Ok(());
        };
        win32_result(unsafe { RegFlushKey(native_key(&key)) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Isolated registry tests must never publish a real desktop notification.
    #[derive(Default)]
    struct IsolatedNotification(u32);

    impl transaction::Notification for IsolatedNotification {
        fn current_session(&mut self) -> io::Result<u32> {
            Ok(42)
        }
        fn logon_time(&mut self, _: u32) -> io::Result<Option<i64>> {
            Ok(Some(100))
        }
        fn read(&mut self, _: u32) -> io::Result<Option<u32>> {
            Ok(Some(self.0))
        }
        fn publish(&mut self, _: u32, value: u32) -> io::Result<()> {
            self.0 = value;
            Ok(())
        }
    }

    fn temporary_journal(directory: &std::path::Path) -> FileJournal {
        FileJournal {
            directory: directory.to_path_buf(),
            path: directory.join("journal.jsonl"),
            lock: None,
        }
    }

    #[test]
    fn incomplete_snapshot_is_not_published_and_can_be_retried() {
        let temporary = tempfile::tempdir().unwrap();
        let mut journal = temporary_journal(temporary.path());
        let snapshot = b"complete snapshot\n";
        let error = journal
            .create_with(snapshot, |file, contents| {
                file.write_all(&contents[..contents.len() / 2])?;
                file.sync_all()?;
                Err(io::Error::other("simulated snapshot sync failure"))
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "simulated snapshot sync failure");
        assert!(!journal.path.exists());
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 0);

        journal.create(snapshot).unwrap();
        assert_eq!(fs::read(&journal.path).unwrap(), snapshot);
    }

    #[test]
    fn snapshot_creation_never_replaces_an_existing_journal() {
        let temporary = tempfile::tempdir().unwrap();
        let mut journal = temporary_journal(temporary.path());
        fs::write(&journal.path, b"existing journal").unwrap();
        assert_eq!(
            journal.create(b"new snapshot\n").unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );
        assert_eq!(fs::read(&journal.path).unwrap(), b"existing journal");
        assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_intent_append_removes_only_its_own_partial_tail() {
        let temporary = tempfile::tempdir().unwrap();
        let mut journal = temporary_journal(temporary.path());
        journal.create(b"snapshot\nprevious intent\n").unwrap();
        let error = journal
            .append_with(b"next intent\n", |file, contents| {
                file.write_all(&contents[..4])?;
                file.sync_all()?;
                Err(io::Error::other("simulated intent sync failure"))
            })
            .unwrap_err();
        assert_eq!(error.to_string(), "simulated intent sync failure");
        assert_eq!(
            fs::read(&journal.path).unwrap(),
            b"snapshot\nprevious intent\n"
        );
        journal.append(b"next intent\n").unwrap();
        assert_eq!(
            fs::read(&journal.path).unwrap(),
            b"snapshot\nprevious intent\nnext intent\n"
        );
    }

    struct IsolatedRegistry(String);

    impl Drop for IsolatedRegistry {
        fn drop(&mut self) {
            // This path contains a freshly generated UUID and never points to
            // the user's actual TabletTip settings.
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.0);
        }
    }

    #[test]
    fn real_registry_transaction_restores_an_isolated_user_key() {
        let path = IsolatedRegistry(format!(
            r"Software\SunshineFoundationTouchKeyboardTest-{}",
            uuid::Uuid::new_v4()
        ));
        let user = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = user.create_subkey(&path.0).unwrap();
        key.set_value("EnableDesktopModeAutoInvoke", &7u32).unwrap();
        key.set_value("TouchKeyboardTapInvoke", &"original")
            .unwrap();
        drop(key);

        let mut probe = UserRegistry {
            path: path.0.clone(),
        };
        let original_enabled = probe.read("EnableDesktopModeAutoInvoke").unwrap();
        let original_tap = probe.read("TouchKeyboardTapInvoke").unwrap();
        assert!(
            probe
                .read("PreventTouchKeyboardAutoInvoke")
                .unwrap()
                .is_none()
        );

        let temporary = tempfile::tempdir().unwrap();
        let journal = FileJournal {
            directory: temporary.path().to_path_buf(),
            path: temporary.path().join("journal.jsonl"),
            lock: None,
        };
        let mut transaction = Transaction::new(
            UserRegistry {
                path: path.0.clone(),
            },
            journal,
            IsolatedNotification::default(),
        );
        transaction.reconcile(Some(false), 42).unwrap();
        assert_eq!(
            probe.read("EnableDesktopModeAutoInvoke").unwrap(),
            original_enabled
        );
        assert!(
            probe
                .read("PreventTouchKeyboardAutoInvoke")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            probe.read("TouchKeyboardTapInvoke").unwrap(),
            Some(RawValue {
                kind: 4,
                bytes: 0u32.to_le_bytes().to_vec()
            })
        );
        transaction.reconcile(None, 42).unwrap();
        assert_eq!(
            probe.read("EnableDesktopModeAutoInvoke").unwrap(),
            original_enabled
        );
        assert_eq!(probe.read("TouchKeyboardTapInvoke").unwrap(), original_tap);
        assert!(
            probe
                .read("PreventTouchKeyboardAutoInvoke")
                .unwrap()
                .is_none()
        );
        assert!(!temporary.path().join("journal.jsonl").exists());
    }

    #[test]
    fn requires_capability_and_matching_process_session() {
        let mut state = crate::sunshine::TrayState::default();
        state.touch_keyboard = Some(crate::sunshine::TrayTouchKeyboard {
            enabled: true,
            session_id: 42,
        });
        assert_eq!(requested_mode(Some(&state), Some(42)), None);
        state.capabilities.push(CAPABILITY.into());
        assert_eq!(requested_mode(Some(&state), Some(42)), Some(true));
        assert_eq!(requested_mode(Some(&state), Some(41)), None);
        assert_eq!(requested_mode(Some(&state), None), None);
        state.touch_keyboard.as_mut().unwrap().enabled = false;
        assert_eq!(requested_mode(Some(&state), Some(42)), Some(false));
        state.touch_keyboard = None;
        assert_eq!(requested_mode(Some(&state), Some(42)), None);
        assert_eq!(requested_mode(None, Some(42)), None);
    }
}
