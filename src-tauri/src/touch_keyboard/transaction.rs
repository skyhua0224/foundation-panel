//! Crash-recoverable, per-user transaction. This module knows no registry path.

use serde::{Deserialize, Serialize};
use std::io;

pub(super) const MAX_VALUE_BYTES: usize = 1024 * 1024;
pub(super) const MAX_JOURNAL_BYTES: u64 = 48 * 1024 * 1024;

struct Definition {
    name: &'static str,
    enabled: u32,
    disabled: u32,
}

// The Windows touch keyboard setting used by current transactions.
const KEYS: [Definition; 1] = [Definition {
    name: "TouchKeyboardTapInvoke",
    enabled: 2,
    disabled: 0,
}];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct RawValue {
    pub kind: u32,
    pub bytes: Vec<u8>,
}

fn dword(value: u32) -> RawValue {
    RawValue {
        kind: 4,
        bytes: value.to_le_bytes().to_vec(),
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    name: String,
    original: Option<RawValue>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u32,
    session_id: u32,
    logon_time: i64,
    enabled: bool,
    values: Vec<Entry>,
    /// `None` records that Windows had not published the runtime value
    /// before the session; the session initialized it instead.
    notification_original: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    index: usize,
}

fn desired(snapshot: &Snapshot, index: usize) -> Option<RawValue> {
    if index != 0 {
        return None;
    }
    let definition = &KEYS[index];
    Some(dword(if snapshot.enabled {
        definition.enabled
    } else {
        definition.disabled
    }))
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

/// `create` and `append` must write, flush and sync before returning success.
/// A failed `create` must not leave a partial snapshot at the journal path.
pub(super) trait Journal {
    fn lock(&mut self) -> io::Result<()>;
    fn unlock(&mut self);
    fn read(&mut self) -> io::Result<Option<Vec<u8>>>;
    fn create(&mut self, contents: &[u8]) -> io::Result<()>;
    fn append(&mut self, contents: &[u8]) -> io::Result<()>;
    fn remove(&mut self) -> io::Result<()>;
}

pub(super) trait Registry {
    fn read(&mut self, name: &str) -> io::Result<Option<RawValue>>;
    fn set(&mut self, name: &str, value: &RawValue) -> io::Result<()>;
    fn delete(&mut self, name: &str) -> io::Result<()>;
    fn flush(&mut self) -> io::Result<()>;
}

/// The runtime preference is session-scoped and may differ from the registry.
/// Implementations must reject a different session and unsupported payloads.
/// `read` returns `Ok(None)` when Windows has not published the state yet,
/// which is normal before the first settings write after a reboot.
pub(super) trait Notification {
    fn logon_time(&mut self, session_id: u32) -> io::Result<Option<i64>>;
    fn current_session(&mut self) -> io::Result<u32>;
    fn read(&mut self, session_id: u32) -> io::Result<Option<u32>>;
    fn publish(&mut self, session_id: u32, value: u32) -> io::Result<()>;
}

// The final intent is a notification, after all registry intents.
const NOTIFICATION_INDEX: usize = KEYS.len();

fn notification_target(enabled: bool) -> u32 {
    if enabled { 2 } else { 0 }
}

/// Maps the durable registry preference to the runtime notification value.
/// An absent registry value is the untouched Windows default ("Never"); a
/// value outside 0..=2 cannot be mapped and leaves the runtime unchanged.
fn derived_notification(value: Option<&RawValue>) -> Option<u32> {
    let Some(value) = value else {
        return Some(0);
    };
    let raw = match (value.kind, value.bytes.as_slice().try_into()) {
        (4, Ok(bytes)) => u32::from_le_bytes(bytes),
        _ => return None,
    };
    (raw <= 2).then_some(raw)
}

struct ParsedJournal {
    snapshot: Snapshot,
    intents: Vec<usize>,
}

fn parse_journal(bytes: &[u8]) -> io::Result<ParsedJournal> {
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err(invalid("touch keyboard journal is too large"));
    }
    let header_end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .ok_or_else(|| invalid("touch keyboard journal has no complete snapshot"))?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes[..header_end])
        .map_err(|_| invalid("touch keyboard journal snapshot is invalid"))?;
    let valid_version = snapshot.version == 1
        && snapshot.logon_time > 0
        && !snapshot
            .notification_original
            .is_some_and(|value| value > 2);
    if !valid_version || snapshot.values.len() != KEYS.len() {
        return Err(invalid(
            "touch keyboard journal version or value count is invalid",
        ));
    }
    for (entry, definition) in snapshot.values.iter().zip(KEYS.iter()) {
        if entry.name != definition.name
            || entry
                .original
                .as_ref()
                .is_some_and(|value| value.bytes.len() > MAX_VALUE_BYTES)
        {
            return Err(invalid("touch keyboard journal value is invalid"));
        }
    }

    let tail = &bytes[header_end + 1..];
    let incomplete_tail = !tail.is_empty() && !tail.ends_with(b"\n");
    let complete_len = if incomplete_tail {
        tail.iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |pos| pos + 1)
    } else {
        tail.len()
    };
    let mut intents = Vec::new();
    if complete_len > 0 {
        // The last byte is the newline terminating the last complete intent.
        for line in tail[..complete_len - 1].split(|byte| *byte == b'\n') {
            if line.is_empty() {
                return Err(invalid("touch keyboard journal contains an empty intent"));
            }
            let intent: Intent = serde_json::from_slice(line)
                .map_err(|_| invalid("touch keyboard journal intent is invalid"))?;
            if intent.index > NOTIFICATION_INDEX
                || intents.last().is_some_and(|last| *last >= intent.index)
            {
                return Err(invalid("touch keyboard journal intent order is invalid"));
            }
            if intent.index != NOTIFICATION_INDEX {
                let target = desired(&snapshot, intent.index);
                if target.is_none() || target == snapshot.values[intent.index].original {
                    return Err(invalid("touch keyboard journal intent has no write"));
                }
            }
            intents.push(intent.index);
        }
    }
    Ok(ParsedJournal { snapshot, intents })
}

fn line(value: &impl Serialize) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(super) struct Transaction<R: Registry, J: Journal, N: Notification> {
    registry: R,
    journal: J,
    notification: N,
    locked: bool,
    active: Option<bool>,
    needs_restore: bool,
    owns_journal: bool,
}

impl<R: Registry, J: Journal, N: Notification> Transaction<R, J, N> {
    pub(super) fn new(registry: R, journal: J, notification: N) -> Self {
        Self {
            registry,
            journal,
            notification,
            locked: false,
            active: None,
            needs_restore: false,
            owns_journal: false,
        }
    }

    fn lock(&mut self) -> io::Result<()> {
        if !self.locked {
            self.journal.lock()?;
            self.locked = true;
        }
        Ok(())
    }

    fn unlock(&mut self) {
        if self.locked {
            self.journal.unlock();
            self.locked = false;
        }
    }

    pub(super) fn recover_on_start(&mut self) -> io::Result<()> {
        self.lock()?;
        let result = self.restore_locked();
        self.unlock();
        result
    }

    /// Applies only the current, already session-checked mode. A mode change
    /// completes the previous transaction before taking a new snapshot.
    pub(super) fn reconcile(&mut self, mode: Option<bool>, session_id: u32) -> io::Result<()> {
        if self.locked {
            if !self.needs_restore && self.active.is_some() && self.active == mode {
                return Ok(());
            }
            if let Err(error) = self.restore_locked() {
                self.needs_restore = true;
                return Err(error);
            }
            self.active = None;
            self.needs_restore = false;
            self.unlock();
            if mode.is_none() {
                return Ok(());
            }
        }
        let Some(enabled) = mode else {
            // A prior startup recovery may have failed transiently. Retry on
            // disconnect and other inactive states without starting a change.
            self.lock()?;
            let result = self.restore_locked();
            self.unlock();
            return result;
        };
        self.lock()?;
        if let Err(error) = self.restore_locked() {
            self.unlock();
            return Err(error);
        }

        let result = self.begin_locked(enabled, session_id);
        match result {
            Ok(()) => {
                self.active = Some(enabled);
                Ok(())
            }
            Err(error) => {
                // Some writes may have completed. If rollback fails, retain
                // both the journal and the lock for the next retry or exit.
                match self.restore_locked() {
                    Ok(()) => self.unlock(),
                    Err(restore_error) => {
                        self.needs_restore = true;
                        log::warn!("touch keyboard rollback deferred: {restore_error}");
                    }
                }
                Err(error)
            }
        }
    }

    fn begin_locked(&mut self, enabled: bool, session_id: u32) -> io::Result<()> {
        // Probe before any write. An unavailable internal API must not leave
        // registry-only changes that Windows never applies. `None` records
        // an unpublished state; publishing below initializes it.
        let notification_original = self.notification.read(session_id)?;
        let logon_time = self
            .notification
            .logon_time(session_id)?
            .ok_or_else(|| io::Error::other("touch keyboard Windows session ended"))?;
        let mut values = Vec::with_capacity(KEYS.len());
        for definition in &KEYS {
            let original = self.registry.read(definition.name)?;
            if original
                .as_ref()
                .is_some_and(|value| value.bytes.len() > MAX_VALUE_BYTES)
            {
                return Err(invalid(
                    "touch keyboard registry value is too large to snapshot",
                ));
            }
            values.push(Entry {
                name: definition.name.to_string(),
                original,
            });
        }
        let snapshot = Snapshot {
            version: 1,
            session_id,
            logon_time,
            enabled,
            values,
            notification_original,
        };
        let snapshot_line = line(&snapshot)?;
        if snapshot_line.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(invalid("touch keyboard snapshot is too large"));
        }
        self.journal.create(&snapshot_line)?;
        self.owns_journal = true;

        let mut registry_written = false;
        for (index, entry) in snapshot.values.iter().enumerate() {
            let Some(target) = desired(&snapshot, index) else {
                continue;
            };
            if Some(&target) == entry.original.as_ref() {
                continue;
            }
            if self.registry.read(&entry.name)? != entry.original {
                return Err(io::Error::other(format!(
                    "touch keyboard value changed before write: {}",
                    entry.name
                )));
            }
            // The intent is durable before the corresponding registry write.
            // This makes a partially applied snapshot recoverable after a crash.
            self.journal.append(&line(&Intent { index })?)?;
            self.registry.set(&entry.name, &target)?;
            registry_written = true;
        }
        if registry_written {
            self.registry.flush()?;
        }
        if self.notification.read(session_id)? != notification_original {
            return Err(io::Error::other(
                "touch keyboard notification changed before publish",
            ));
        }
        // A crash after publishing must still be able to restore the runtime
        // value, including when all registry values already matched the target.
        self.journal.append(&line(&Intent {
            index: NOTIFICATION_INDEX,
        })?)?;
        self.publish_verified(session_id, notification_target(enabled))?;
        Ok(())
    }

    fn publish_verified(&mut self, session_id: u32, value: u32) -> io::Result<()> {
        self.notification.publish(session_id, value)?;
        if self.notification.read(session_id)? != Some(value) {
            return Err(io::Error::other(
                "touch keyboard notification readback mismatch",
            ));
        }
        Ok(())
    }

    fn restore_locked(&mut self) -> io::Result<()> {
        let Some(bytes) = self.journal.read()? else {
            if self.owns_journal {
                return Err(io::Error::other("active touch keyboard journal is missing"));
            }
            return Ok(());
        };
        let parsed = parse_journal(&bytes)?;
        // Check before touching the shared user registry: a different live
        // session may still own the runtime preference after its GUI crashed.
        let mut restore_notification = true;
        if parsed.intents.contains(&NOTIFICATION_INDEX) {
            let session_id = parsed.snapshot.session_id;
            let current_session = self.notification.current_session()?;
            let actual_logon = self.notification.logon_time(session_id)?;
            let same_logon = actual_logon == Some(parsed.snapshot.logon_time);
            if !same_logon {
                restore_notification = false;
            } else if current_session != session_id {
                return Err(io::Error::other(
                    "touch keyboard recovery belongs to another active Windows session",
                ));
            }
        }
        let mut first_error = None;
        for &index in parsed.intents.iter().rev() {
            if index == NOTIFICATION_INDEX {
                continue;
            }
            let entry = &parsed.snapshot.values[index];
            let target = desired(&parsed.snapshot, index);
            let current = match self.registry.read(&entry.name) {
                Ok(value) => value,
                Err(error) => {
                    first_error.get_or_insert(error);
                    continue;
                }
            };
            if current == entry.original {
                continue;
            }
            if current != target {
                log::warn!(
                    "preserving externally changed touch keyboard value: {}",
                    entry.name
                );
                continue;
            }
            let result = match &entry.original {
                Some(original) => self.registry.set(&entry.name, original),
                None => self.registry.delete(&entry.name),
            };
            if let Err(error) = result {
                first_error.get_or_insert(error);
            }
        }
        // Flush even when all values already look restored: an earlier restore
        // may have written them successfully but failed its flush.
        if parsed
            .intents
            .iter()
            .any(|index| *index != NOTIFICATION_INDEX)
        {
            if let Err(error) = self.registry.flush() {
                first_error.get_or_insert(error);
            }
        }
        if let Some(error) = first_error {
            return Err(error);
        }
        if restore_notification && parsed.intents.contains(&NOTIFICATION_INDEX) {
            let session_id = parsed.snapshot.session_id;
            let current = self.notification.read(session_id)?;
            match parsed.snapshot.notification_original {
                Some(original) => {
                    if current != Some(original) {
                        if current != Some(notification_target(parsed.snapshot.enabled)) {
                            log::warn!("preserving externally changed touch keyboard notification");
                        } else {
                            // Keep the journal if either publication or
                            // verification fails.
                            self.publish_verified(session_id, original)?;
                        }
                    }
                }
                None => {
                    // The session initialized an unpublished state. Restore
                    // the runtime value from the durable registry preference.
                    let target = notification_target(parsed.snapshot.enabled);
                    if current == Some(target) {
                        let current_registry_value =
                            self.registry.read(&parsed.snapshot.values[0].name)?;
                        match derived_notification(current_registry_value.as_ref()) {
                            Some(derived) => self.publish_verified(session_id, derived)?,
                            None => log::warn!("cannot derive the touch keyboard notification; leaving it unchanged"),
                        }
                    } else if current.is_some() {
                        log::warn!("preserving externally changed touch keyboard notification");
                    }
                }
            }
        }
        // An incomplete intent cannot have passed sync before its write.
        self.journal.remove()?;
        self.owns_journal = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
