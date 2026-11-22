//! VB-CABLE component management. Download, device inspection and driver
//! changes are separate operations; Sunshine only supplies audio status.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

const ARCHIVE_NAME: &str = "VBCABLE_Driver_Pack43.zip";
const ARCHIVE_URL: &str = "https://download.vb-audio.com/Download_CABLE/VBCABLE_Driver_Pack43.zip";
const ARCHIVE_SHA256: &str = "66fd0a4d9f4896ff41632b7e3d53892c085c4561f53e8ae8d0f0bc10eedd1cdd";
const MAX_ARCHIVE_BYTES: u64 = 32 * 1024 * 1024;
const DRIVER_OPERATION_TIMEOUT: Duration = Duration::from_secs(3 * 60);
#[cfg(target_os = "windows")]
const DEVICE_PROBE_TIMEOUT: Duration = Duration::from_secs(30);
static OPERATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn wait_for_driver_operation<F, T>(operation: F, timeout: Duration) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| "VBCABLE-OP-TIMEOUT: the driver operation may still be running".to_string())?
}

fn pack43_supports_native_machine(machine: u16) -> bool {
    machine == 0x8664 // IMAGE_FILE_MACHINE_AMD64
}

#[cfg(target_os = "windows")]
fn ensure_pack43_architecture() -> Result<(), String> {
    use windows::Win32::System::SystemInformation::IMAGE_FILE_MACHINE;
    use windows::Win32::System::Threading::{GetCurrentProcess, IsWow64Process2};

    let mut process_machine = IMAGE_FILE_MACHINE::default();
    let mut native_machine = IMAGE_FILE_MACHINE::default();
    unsafe {
        IsWow64Process2(
            GetCurrentProcess(),
            &mut process_machine,
            Some(&mut native_machine),
        )
    }
    .map_err(|error| format!("VBCABLE-ARCH-UNAVAILABLE: {error}"))?;
    if !pack43_supports_native_machine(native_machine.0) {
        return Err("VBCABLE-ARCH-UNSUPPORTED: Pack43 requires x64 Windows".into());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn ensure_pack43_architecture() -> Result<(), String> {
    Err("VB-CABLE is supported only on Windows".into())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// missing, installed, disabled, unavailable, or pending (driver without a device node).
    pub state: String,
    pub archive_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    pub status: Option<ComponentStatus>,
    pub check_error: Option<String>,
    /// Windows reported 3010. This is informational and never blocks another operation.
    pub restart_suggested: bool,
}

fn archive_is_valid(path: &Path) -> bool {
    crate::native_components::install::sha256_file(path, MAX_ARCHIVE_BYTES)
        .is_ok_and(|actual| actual == ARCHIVE_SHA256)
}

fn cached_archive() -> Result<PathBuf, String> {
    Ok(dirs::cache_dir()
        .ok_or("VB-CABLE: the download directory is unavailable")?
        .join("Sunshine")
        .join("vb-cable")
        .join(ARCHIVE_NAME))
}

fn usable_archive() -> Option<PathBuf> {
    let cached = cached_archive().ok()?;
    archive_is_valid(&cached).then_some(cached)
}

// A cancelled download or a failed cache publication must not leave a new
// UUID-named partial file behind. A successful rename leaves nothing to remove.
struct TemporaryDownload(PathBuf);

impl TemporaryDownload {
    async fn publish(self, destination: &Path) -> Result<(), String> {
        if destination.exists() {
            tokio::fs::remove_file(destination)
                .await
                .map_err(|error| format!("VB-CABLE: cannot replace cached archive: {error}"))?;
        }
        tokio::fs::rename(&self.0, destination)
            .await
            .map_err(|error| format!("VB-CABLE: cannot save downloaded archive: {error}"))
    }
}

impl Drop for TemporaryDownload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(target_os = "windows")]
fn system_powershell_modules() -> PathBuf {
    crate::elevation::system_binary(r"WindowsPowerShell\v1.0\Modules")
}

#[cfg(target_os = "windows")]
pub(crate) mod probe;

#[cfg(target_os = "windows")]
use probe::probe_device;

#[cfg(not(target_os = "windows"))]
async fn probe_device() -> Result<String, String> {
    Err("VB-CABLE is supported only on Windows".into())
}

#[cfg(target_os = "windows")]
async fn wait_for_device_probe(
    mut child: tokio::process::Child,
    budget: Duration,
) -> Result<Vec<u8>, String> {
    use tokio::io::AsyncReadExt;
    let stdout = child
        .stdout
        .take()
        .ok_or("VB-CABLE: device check has no output")?;
    let result = tokio::time::timeout(budget, async {
        // pnputil can return more than one pipe buffer. Discard an incomplete
        // snapshot rather than reporting a missing driver.
        const MAX_OUTPUT: usize = 16 * 1024 * 1024;
        let read = async {
            let mut bytes = Vec::new();
            stdout
                .take((MAX_OUTPUT + 1) as u64)
                .read_to_end(&mut bytes)
                .await?;
            if bytes.len() > MAX_OUTPUT {
                return Err(std::io::Error::other("device check output is too large"));
            }
            Ok(bytes)
        };
        let (bytes, status) = tokio::try_join!(read, child.wait())?;
        Ok::<_, std::io::Error>((status, bytes))
    })
    .await;
    match result {
        Ok(Ok((status, bytes))) if status.success() => Ok(bytes),
        Ok(Ok(_)) => Err("VB-CABLE: unable to check the device and driver service".into()),
        Ok(Err(error)) => Err(format!("VB-CABLE: device check failed: {error}")),
        Err(_) => {
            // This child only reads device state; never terminate the elevated
            // helper that may be changing a driver.
            let _ = child.kill().await;
            let _ = child.wait().await;
            Err("VB-CABLE: device check timed out".into())
        }
    }
}

async fn read_status() -> Result<ComponentStatus, String> {
    let state = probe_device().await?;
    let archive_ready = tokio::task::spawn_blocking(|| usable_archive().is_some())
        .await
        .map_err(|error| format!("VB-CABLE: archive check failed: {error}"))?;
    Ok(ComponentStatus {
        state,
        archive_ready,
    })
}

/// Check only; neither a download nor a driver operation is started.
#[tauri::command]
pub async fn vb_cable_check() -> Result<ComponentStatus, String> {
    read_status().await
}

/// Download only. The pinned digest is checked before publishing the cache.
#[tauri::command]
pub async fn vb_cable_download() -> Result<(), String> {
    let _guard = OPERATION.lock().await;
    if usable_archive().is_some() {
        return Ok(());
    }
    let destination = cached_archive()?;
    let parent = destination
        .parent()
        .ok_or("VB-CABLE: invalid cache directory")?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("VB-CABLE: cannot prepare download directory: {error}"))?;
    let partial =
        TemporaryDownload(parent.join(format!("download-{}.partial", uuid::Uuid::new_v4())));
    let request = crate::github_download::DownloadRequest {
        url: ARCHIVE_URL,
        destination: &partial.0,
        expected_size: None,
        expected_sha256: Some(ARCHIVE_SHA256),
        max_size: Some(MAX_ARCHIVE_BYTES),
        user_agent: "Sunshine Control Panel",
        connect_timeout: crate::github_download::DEFAULT_CONNECT_TIMEOUT,
        response_timeout: crate::github_download::DEFAULT_RESPONSE_TIMEOUT,
        idle_timeout: crate::github_download::DEFAULT_IDLE_TIMEOUT,
        overall_timeout: Some(Duration::from_secs(180)),
    };
    let candidates = [crate::github_download::DownloadCandidate {
        name: "VB-Audio",
        url: ARCHIVE_URL.to_string(),
    }];
    let outcome =
        crate::github_download::download_to_file_from_candidates(request, &candidates, |_| {})
            .await;
    if let Err(error) = outcome {
        return Err(format!("VB-CABLE: download failed: {error}"));
    }
    partial.publish(&destination).await
}

async fn ensure_no_streams() -> Result<(), String> {
    // A stopped host cannot own a stream. When any Sunshine process is up,
    // require a valid local sessions response; 404/XML is not an empty list.
    if !sunshine_process_running()? {
        return Ok(());
    }
    let url = crate::sunshine::get_local_sunshine_url().await?;
    let client = crate::sunshine::create_https_client()?;
    let response = client
        .get(format!(
            "{}/api/runtime/sessions",
            url.trim_end_matches('/')
        ))
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .map_err(|error| format!("VB-CABLE: unable to check active streams: {error}"))?;
    if !response.status().is_success() {
        return Err("VB-CABLE: unable to check active streams on the local host".into());
    }
    let payload: serde_json::Value = response
        .json()
        .await
        .map_err(|error| format!("VB-CABLE: invalid stream status: {error}"))?;
    let sessions = payload
        .get("sessions")
        .and_then(|value| value.as_array())
        .filter(|_| payload.get("success").and_then(|value| value.as_bool()) == Some(true))
        .ok_or("VB-CABLE: invalid stream status")?;
    if !sessions.is_empty() {
        return Err("VB-CABLE: finish the active Sunshine stream first".into());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn sunshine_process_running() -> Result<bool, String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW,
        TH32CS_SNAPPROCESS,
    };
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
        .map_err(|error| format!("VB-CABLE: cannot check Sunshine processes: {error}"))?;
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    if let Err(error) = unsafe { Process32FirstW(snapshot, &mut entry) } {
        unsafe {
            let _ = CloseHandle(snapshot);
        }
        return Err(format!("VB-CABLE: cannot read Sunshine processes: {error}"));
    }
    let mut more = true;
    let mut found = false;
    while more {
        let end = entry
            .szExeFile
            .iter()
            .position(|&ch| ch == 0)
            .unwrap_or(entry.szExeFile.len());
        if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("sunshine.exe") {
            found = true;
            break;
        }
        more = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
    }
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    Ok(found)
}

#[cfg(not(target_os = "windows"))]
fn sunshine_process_running() -> Result<bool, String> {
    Ok(false)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Request {
    Install { archive: PathBuf },
    Enable,
    Uninstall,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    code: i32,
    error: String,
}

#[tauri::command]
pub async fn vb_cable_install() -> Result<OperationResult, String> {
    let _guard = OPERATION.lock().await;
    ensure_no_streams().await?;
    let status = read_status().await?;
    if status.state != "missing" {
        return Err("VB-CABLE: an existing or pending device must be resolved first".into());
    }
    ensure_pack43_architecture()?;
    let archive = usable_archive().ok_or("VB-CABLE: download the verified package first")?;
    let code = run_elevated(Request::Install { archive }).await?;
    let status = read_status().await;
    if code == 0 && status.as_ref().map(|s| s.state.as_str()) != Ok("installed") {
        return Err("VB-CABLE: the installer finished but the device could not be verified".into());
    }
    let (status, check_error) = match status {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error)),
    };
    Ok(OperationResult {
        status,
        check_error,
        restart_suggested: code == 3010,
    })
}

#[tauri::command]
pub async fn vb_cable_uninstall() -> Result<OperationResult, String> {
    let _guard = OPERATION.lock().await;
    ensure_no_streams().await?;
    let status = read_status().await?;
    if !matches!(
        status.state.as_str(),
        "installed" | "disabled" | "unavailable" | "pending"
    ) {
        return Err("VB-CABLE: no removable device was found".into());
    }
    let code = run_elevated(Request::Uninstall).await?;
    let status = read_status().await;
    if code == 0 && status.as_ref().map(|s| s.state.as_str()) != Ok("missing") {
        return Err(
            "VB-CABLE: removal finished but the device could not be verified as absent".into(),
        );
    }
    let (status, check_error) = match status {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error)),
    };
    Ok(OperationResult {
        status,
        check_error,
        restart_suggested: code == 3010,
    })
}

#[tauri::command]
pub async fn vb_cable_enable() -> Result<OperationResult, String> {
    let _guard = OPERATION.lock().await;
    ensure_no_streams().await?;
    if read_status().await?.state != "disabled" {
        return Err("VB-CABLE: no disabled device was found".into());
    }
    let code = run_elevated(Request::Enable).await?;
    let mut status = read_status().await;
    for _ in 0..10 {
        if code != 0 || status.as_ref().is_ok_and(|s| s.state == "installed") {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
        status = read_status().await;
    }
    if code == 0 && status.as_ref().map(|s| s.state.as_str()) != Ok("installed") {
        return Err("VB-CABLE: the device could not be verified after enabling".into());
    }
    let (status, check_error) = match status {
        Ok(status) => (Some(status), None),
        Err(error) => (None, Some(error)),
    };
    Ok(OperationResult {
        status,
        check_error,
        restart_suggested: code == 3010,
    })
}

#[cfg(target_os = "windows")]
const HELPER_ARG: &str = "--vb-cable-elevated";

#[cfg(target_os = "windows")]
async fn run_elevated(request: Request) -> Result<i32, String> {
    // Bound the UI wait, but do not terminate an in-progress driver change.
    // The elevated helper owns the machine-wide lock until it actually ends.
    wait_for_driver_operation(
        async {
            let token = uuid::Uuid::new_v4();
            let mut server = crate::elevation::create_hardened_pipe(
                &crate::elevation::pipe_name("vbcable", token),
                "VB-CABLE-UAC",
            )?;
            let payload = serde_json::to_string(&request).map_err(|error| error.to_string())?;
            let process = crate::elevation::spawn_helper(
                vec![HELPER_ARG.into(), token.to_string(), payload],
                "VB-CABLE-UAC",
            )
            .await?;
            crate::elevation::connect_verified(
                &mut server,
                &process,
                Duration::from_secs(30),
                "VB-CABLE-UAC",
            )
            .await?;
            let response = read_helper_response(server).await;
            let exit = loop {
                if let Some(code) =
                    crate::elevation::wait_for_exit(&process, Duration::from_secs(30)).await?
                {
                    break code;
                }
            };
            let response = response?;
            if exit != 0 {
                return Err(format!("VB-CABLE: helper exited with {exit}"));
            }
            if !response.error.is_empty() {
                return Err(response.error);
            }
            Ok(response.code)
        },
        DRIVER_OPERATION_TIMEOUT,
    )
    .await
}

#[cfg(target_os = "windows")]
async fn read_helper_response(
    server: tokio::net::windows::named_pipe::NamedPipeServer,
) -> Result<Response, String> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let mut reader = BufReader::new(server.take(16 * 1024));
    let mut bytes = Vec::new();
    reader
        .read_until(b'\n', &mut bytes)
        .await
        .map_err(|error| format!("VB-CABLE: helper response failed: {error}"))?;
    if bytes.last() != Some(&b'\n') {
        return Err("VB-CABLE: incomplete helper response".into());
    }
    // Acknowledge before the helper closes the pipe; this also preserves
    // short error responses when the helper finishes immediately.
    let _ = reader.get_mut().get_mut().write_all(&[6]).await;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("VB-CABLE: invalid helper response: {error}"))
}

#[cfg(target_os = "windows")]
async fn send_helper_response(
    pipe: &mut tokio::net::windows::named_pipe::NamedPipeClient,
    response: &Response,
) -> Result<(), String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let mut bytes = serde_json::to_vec(response).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    pipe.write_all(&bytes)
        .await
        .map_err(|error| error.to_string())?;
    let mut ack = [0u8; 1];
    // The GUI can time out or exit; do not keep a completed helper alive.
    let _ = tokio::time::timeout(Duration::from_secs(2), pipe.read_exact(&mut ack)).await;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn run_elevated(_request: Request) -> Result<i32, String> {
    Err("VB-CABLE is supported only on Windows".into())
}

// The helper, rather than the GUI, owns this lock while it changes the
// driver. It therefore survives the GUI being closed or crashing.
#[cfg(target_os = "windows")]
pub(crate) struct DriverOperationGuard(windows::Win32::Foundation::HANDLE);

#[cfg(target_os = "windows")]
impl Drop for DriverOperationGuard {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Threading::ReleaseMutex(self.0);
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn lock_driver_operation() -> Result<DriverOperationGuard, String> {
    use windows::Win32::Foundation::{
        CloseHandle, HLOCAL, LocalFree, WAIT_ABANDONED, WAIT_OBJECT_0, WAIT_TIMEOUT,
    };
    use windows::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows::Win32::System::Threading::{CreateMutexW, ReleaseMutex, WaitForSingleObject};
    use windows::core::w;

    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            w!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)"),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .map_err(|error| format!("VB-CABLE: cannot secure the driver operation: {error}"))?;
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor.0,
        bInheritHandle: false.into(),
    };
    let handle = unsafe {
        let result = CreateMutexW(
            Some(&attributes),
            false,
            w!("Global\\Sunshine-VBCable-DriverOperation"),
        );
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        result
    }
    .map_err(|error| {
        if error.code() == windows::core::HRESULT::from_win32(5) {
            format!("VBCABLE-OP-ACCESS-DENIED: {error}")
        } else {
            format!("VB-CABLE: cannot coordinate driver operations: {error}")
        }
    })?;
    match unsafe { WaitForSingleObject(handle, 0) } {
        WAIT_OBJECT_0 => Ok(DriverOperationGuard(handle)),
        WAIT_ABANDONED => {
            unsafe {
                let _ = ReleaseMutex(handle);
                let _ = CloseHandle(handle);
            }
            Err("VB-CABLE: a previous driver operation was interrupted; check the device before retrying".into())
        }
        wait => {
            unsafe {
                let _ = CloseHandle(handle);
            }
            if wait == WAIT_TIMEOUT {
                Err("VBCABLE-OP-BUSY: another driver operation is still running".into())
            } else {
                Err(format!(
                    "VB-CABLE: cannot coordinate driver operations: {wait:?}"
                ))
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn run_script(request: &Request) -> Result<i32, String> {
    use base64::Engine;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    // The staging directory has a protected DACL. Embedded scripts are never
    // loaded from the writable portable installation or a user temp folder.
    const CREATE_STAGE: &str = r#"
$ErrorActionPreference = 'Stop'
$base = [Environment]::GetFolderPath('CommonApplicationData')
if (-not $base) { throw 'ProgramData unavailable' }
$dir = Join-Path $base ('Sunshine-VBCable-' + [guid]::NewGuid().ToString('N'))
if (Test-Path -LiteralPath $dir) { throw 'staging directory exists' }
$acl = New-Object Security.AccessControl.DirectorySecurity
$acl.SetSecurityDescriptorSddlForm('D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)')
[IO.Directory]::CreateDirectory($dir, $acl) | Out-Null
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
[Console]::WriteLine($base)
[Console]::Write($dir)
"#;
    let powershell = crate::elevation::system_binary(r"WindowsPowerShell\v1.0\powershell.exe");
    let output = Command::new(powershell)
        .args(["-NoProfile", "-NonInteractive", "-Command", CREATE_STAGE])
        .env("PSModulePath", system_powershell_modules())
        .stdin(Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .map_err(|error| format!("VB-CABLE: cannot create protected staging: {error}"))?;
    if !output.status.success() {
        return Err("VB-CABLE: cannot create protected staging".into());
    }
    let paths = String::from_utf8(output.stdout).map_err(|_| "VB-CABLE: invalid staging path")?;
    let mut paths = paths.lines();
    let base = std::fs::canonicalize(paths.next().ok_or("VB-CABLE: missing staging base")?)
        .map_err(|_| "VB-CABLE: invalid staging base")?;
    let stage = PathBuf::from(paths.next().ok_or("VB-CABLE: missing staging path")?);
    let metadata =
        std::fs::symlink_metadata(&stage).map_err(|_| "VB-CABLE: invalid staging path")?;
    use std::os::windows::fs::MetadataExt;
    if paths.next().is_some()
        || !stage.is_absolute()
        || metadata.file_attributes() & 0x400 != 0
        || !stage
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("Sunshine-VBCable-"))
        || std::fs::canonicalize(&stage)
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .as_ref()
            != Some(&base)
    {
        return Err("VB-CABLE: invalid staging path".into());
    }
    struct Stage(PathBuf, PathBuf);
    impl Drop for Stage {
        fn drop(&mut self) {
            use std::os::windows::fs::MetadataExt;
            let safe = std::fs::symlink_metadata(&self.0)
                .is_ok_and(|m| m.file_attributes() & 0x400 == 0)
                && std::fs::canonicalize(&self.0)
                    .ok()
                    .and_then(|p| p.parent().map(Path::to_path_buf))
                    .as_ref()
                    == Some(&self.1);
            if safe {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }
    let stage = Stage(stage, base);
    let files: &[(&str, &[u8])] = match request {
        Request::Install { .. } => &[(
            "install-vsink.bat",
            include_bytes!("../resources/vb-cable/install-vsink.bat"),
        )],
        Request::Enable => &[(
            "enable-vsink.ps1",
            include_bytes!("../resources/vb-cable/enable-vsink.ps1"),
        )],
        Request::Uninstall => &[
            (
                "uninstall-vsink.bat",
                include_bytes!("../resources/vb-cable/uninstall-vsink.bat"),
            ),
            (
                "uninstall-vsink.ps1",
                include_bytes!("../resources/vb-cable/uninstall-vsink.ps1"),
            ),
        ],
    };
    for (name, bytes) in files {
        std::fs::write(stage.0.join(name), bytes)
            .map_err(|error| format!("VB-CABLE: cannot stage operation: {error}"))?;
    }
    let mut command = match request {
        Request::Install { archive } => {
            let wide: Vec<u16> = archive.as_os_str().encode_wide().collect();
            let bytes: Vec<u8> = wide.into_iter().flat_map(u16::to_le_bytes).collect();
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            // cmd treats '=' as an argument separator unless the base64 is quoted.
            let mut command = Command::new(crate::elevation::system_binary("cmd.exe"));
            command
                .args(["/d", "/c"])
                .raw_arg(format!("call install-vsink.bat --component \"{encoded}\""));
            command
        }
        Request::Uninstall => {
            let mut command = Command::new(crate::elevation::system_binary("cmd.exe"));
            command
                .args(["/d", "/c"])
                .raw_arg("call uninstall-vsink.bat --component");
            command
        }
        Request::Enable => {
            let mut command = Command::new(crate::elevation::system_binary(
                r"WindowsPowerShell\v1.0\powershell.exe",
            ));
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "enable-vsink.ps1",
            ]);
            command.env("PSModulePath", system_powershell_modules());
            command
        }
    };
    let result = command
        .current_dir(&stage.0)
        .stdin(Stdio::null())
        .creation_flags(0x08000000)
        .output()
        .map_err(|error| format!("VB-CABLE: driver operation did not start: {error}"))?;
    let code = result.status.code().unwrap_or(-1);
    if code == 0 || code == 3010 {
        Ok(code)
    } else {
        let stderr = String::from_utf8_lossy(&result.stderr);
        let stdout = String::from_utf8_lossy(&result.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        let detail: String = detail.chars().take(2048).collect();
        if detail.is_empty() {
            Err(format!("VB-CABLE: driver operation failed (exit {code})"))
        } else {
            Err(format!(
                "VB-CABLE: driver operation failed (exit {code}): {detail}"
            ))
        }
    }
}

#[cfg(target_os = "windows")]
async fn helper(token: uuid::Uuid, request: Request) -> i32 {
    let Ok(mut pipe) = crate::elevation::connect_helper_pipe(
        &crate::elevation::pipe_name("vbcable", token),
        "VB-CABLE-UAC",
    )
    .await
    else {
        return 3;
    };
    let result = async {
        if !crate::elevation::is_elevated() {
            return Err("VB-CABLE: administrator authorization was not granted".to_string());
        }
        let _driver_operation = lock_driver_operation()?;
        ensure_no_streams().await?;
        let state = probe_device().await?;
        match &request {
            Request::Install { archive } if state == "missing" && archive_is_valid(archive) => {
                ensure_pack43_architecture()?;
            }
            Request::Enable if state == "disabled" => {}
            Request::Uninstall
                if matches!(
                    state.as_str(),
                    "installed" | "disabled" | "unavailable" | "pending"
                ) => {}
            _ => return Err("VB-CABLE: device or package changed before the operation".into()),
        }
        // Device inspection can take time. Recheck the host immediately
        // before starting the driver operation in case a stream began meanwhile.
        ensure_no_streams().await?;
        run_script(&request)
    }
    .await;
    let response = match result {
        Ok(code) => Response {
            code,
            error: String::new(),
        },
        Err(error) => Response { code: 1, error },
    };
    if send_helper_response(&mut pipe, &response).await.is_err() {
        return 4;
    }
    0
}

#[cfg(target_os = "windows")]
pub(crate) fn try_handle_elevated_command() -> Option<i32> {
    let mut args = std::env::args().skip(1);
    if args.next()?.as_str() != HELPER_ARG {
        return None;
    }
    let Some(token) = args
        .next()
        .and_then(|value| uuid::Uuid::parse_str(&value).ok())
    else {
        return Some(2);
    };
    let Some(request) = args
        .next()
        .and_then(|value| serde_json::from_str(&value).ok())
    else {
        return Some(2);
    };
    if args.next().is_some() {
        return Some(2);
    }
    let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return Some(3);
    };
    Some(runtime.block_on(helper(token, request)))
}

#[cfg(test)]
mod tests {
    use super::pack43_supports_native_machine;

    #[tokio::test]
    async fn failed_cache_publication_removes_the_download() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("download.partial");
        let destination = dir.path().join("occupied");
        std::fs::write(&partial, b"verified archive").unwrap();
        std::fs::create_dir(&destination).unwrap();
        let error = super::TemporaryDownload(partial.clone())
            .publish(&destination)
            .await
            .unwrap_err();
        assert!(error.contains("cannot replace cached archive"));
        assert!(!partial.exists());
        assert!(destination.is_dir());
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn timed_out_device_probe_is_terminated() {
        use std::process::Stdio;
        let dir = tempfile::tempdir().unwrap();
        let started = dir.path().join("started");
        let release = dir.path().join("release");
        let completed = dir.path().join("completed");
        let child = tokio::process::Command::new(crate::elevation::system_binary(
            r"WindowsPowerShell\v1.0\powershell.exe",
        ))
        .args([
            "-NoProfile", "-NonInteractive", "-Command",
            "[IO.File]::WriteAllText($env:SUNSHINE_PROBE_STARTED, 'started'); while (-not [IO.File]::Exists($env:SUNSHINE_PROBE_RELEASE)) { Start-Sleep -Milliseconds 100 }; [IO.File]::WriteAllText($env:SUNSHINE_PROBE_COMPLETED, 'completed')",
        ])
        .env("PSModulePath", super::system_powershell_modules())
        .env("SUNSHINE_PROBE_STARTED", &started)
        .env("SUNSHINE_PROBE_RELEASE", &release)
        .env("SUNSHINE_PROBE_COMPLETED", &completed)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .creation_flags(0x08000000)
        .spawn()
        .unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !started.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("probe started");
        let error = super::wait_for_device_probe(child, std::time::Duration::from_millis(100))
            .await
            .unwrap_err();
        assert!(error.contains("device check timed out"));
        std::fs::write(&release, b"release").unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(2200)).await;
        assert!(!completed.exists(), "the timed-out probe must not continue");
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn device_removal_keeps_restart_result_without_deleting_the_package() {
        use std::os::windows::process::CommandExt;
        let script = include_str!("../resources/vb-cable/uninstall-vsink.ps1");
        let (functions, _) = script.split_once("\ntry {").expect("uninstall entry point");
        let mock = r#"
$mockDevice = [pscustomobject]@{ IsVbCable = $true; Present = $true; InstanceId = 'ROOT\MEDIA\0000'; InfName = 'oem42.inf' }
$mockPackage = [pscustomobject]@{ ProviderName = 'VB-Audio Software'; ClassName = 'MEDIA'; Inbox = $false; Driver = 'oem42.inf'; OriginalFileName = 'vbmmecable64_win10.inf' }
try {
  $script:restartSuggested = $false
  Remove-VbCableDrivers { $mockDevice } { $mockPackage } { 3010 } { throw 'package removal must not run' } { $false } { } { }
  if (-not $script:restartSuggested) { throw '3010 was not retained' }
  $script:restartSuggested = $false
  try {
    Remove-VbCableDrivers { $mockDevice } { $mockPackage } { 0 } { throw 'package removal must not run' } { $false } { } { }
    throw 'busy removal incorrectly succeeded'
  } catch {
    if ($_.Exception.Message -notlike 'VBCABLE-DEVICE-BUSY:*') { throw }
  }
} catch { [Console]::Error.Write($_.Exception.Message); exit 1 }
exit 0
"#;
        let output = std::process::Command::new(crate::elevation::system_binary(
            r"WindowsPowerShell\v1.0\powershell.exe",
        ))
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &(functions.to_string() + mock),
        ])
        .env("PSModulePath", super::system_powershell_modules())
        .creation_flags(0x08000000)
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "status: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn failed_package_removal_retries_only_the_identified_absent_device() {
        use std::os::windows::process::CommandExt;
        let script = include_str!("../resources/vb-cable/uninstall-vsink.ps1");
        let (functions, _) = script.split_once("\ntry {").expect("uninstall entry point");
        let mock = r#"
$mockDevice = [pscustomobject]@{ IsVbCable = $true; Present = $false; InstanceId = 'ROOT\MEDIA\0000'; InfName = 'oem42.inf' }
$mockOther = [pscustomobject]@{ IsVbCable = $false; Present = $false; InstanceId = 'ROOT\MEDIA\0001'; InfName = 'oem42.inf' }
$mockPackage = [pscustomobject]@{ ProviderName = 'VB-Audio Software'; ClassName = 'MEDIA'; Inbox = $false; Driver = 'oem42.inf'; OriginalFileName = 'vbmmecable64_win10.inf' }
try {
  $script:ghost = $true; $script:installed = $true; $script:removed = 0; $script:attempts = 0
  Remove-VbCableDrivers { if ($script:ghost) { $mockDevice } } { if ($script:installed) { $mockPackage } } `
    { param($id) if ($id -ne $mockDevice.InstanceId) { throw 'unexpected device' }; $script:removed++; $script:ghost = $false; 0 } `
    { $script:attempts++; if ($script:ghost) { 5 } else { $script:installed = $false; 0 } } { $true } { } { }
  if ($script:removed -ne 1 -or $script:attempts -ne 2 -or $script:installed) { throw 'residual cleanup failed' }
  try {
    Remove-VbCableDrivers { $mockDevice; $mockOther } { $mockPackage } { throw 'device removal must not run' } `
      { throw 'package removal must not run' } { $true } { } { }
    throw 'shared package incorrectly accepted'
  } catch {
    if ($_.Exception.Message -notlike '*also used by another device*') { throw }
  }
  $script:removed = 0
  try {
    Remove-VbCableDrivers { $mockDevice } { $mockPackage } { $script:removed++; 0 } { 5 } { $true } { } { }
    throw 'unchanged residual incorrectly accepted'
  } catch {
    if ($_.Exception.Message -notlike '*could not be safely released*' -or $script:removed -ne 1) { throw }
  }
} catch { [Console]::Error.Write($_.Exception.Message); exit 1 }
exit 0
"#;
        let output = std::process::Command::new(crate::elevation::system_binary(
            r"WindowsPowerShell\v1.0\powershell.exe",
        ))
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &(functions.to_string() + mock),
        ])
        .env("PSModulePath", super::system_powershell_modules())
        .creation_flags(0x08000000)
        .output()
        .unwrap();
        assert!(
            output.status.success(),
            "status: {}\nstdout: {}\nstderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[tokio::test]
    async fn timeout_only_stops_waiting_for_independent_work() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        use std::time::Duration;
        let finished = Arc::new(AtomicBool::new(false));
        let background_finished = Arc::clone(&finished);
        let task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(50)).await;
            background_finished.store(true, Ordering::SeqCst);
        });
        let result = super::wait_for_driver_operation(
            async move { task.await.map_err(|error| error.to_string()) },
            Duration::from_millis(5),
        )
        .await;
        assert!(result.unwrap_err().starts_with("VBCABLE-OP-TIMEOUT:"));
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(finished.load(Ordering::SeqCst));
    }

    #[cfg(target_os = "windows")]
    #[tokio::test]
    async fn immediately_finished_helper_delivers_busy_response() {
        let name = crate::elevation::pipe_name("vbcable-test", uuid::Uuid::new_v4());
        let server = crate::elevation::create_hardened_pipe(&name, "VB-CABLE-TEST").unwrap();
        let client = tokio::spawn(async move {
            let mut pipe = crate::elevation::connect_helper_pipe(&name, "VB-CABLE-TEST")
                .await
                .unwrap();
            super::send_helper_response(
                &mut pipe,
                &super::Response {
                    code: 1,
                    error: "VBCABLE-OP-BUSY: another operation".to_string(),
                },
            )
            .await
            .unwrap();
        });
        server.connect().await.unwrap();
        let response = super::read_helper_response(server).await.unwrap();
        assert!(response.error.starts_with("VBCABLE-OP-BUSY:"));
        client.await.unwrap();
    }

    #[test]
    fn pack43_installation_is_limited_to_native_x64() {
        assert!(pack43_supports_native_machine(0x8664));
        assert!(!pack43_supports_native_machine(0xaa64));
        assert!(!pack43_supports_native_machine(0x014c));
        assert!(!pack43_supports_native_machine(0));
    }
}
