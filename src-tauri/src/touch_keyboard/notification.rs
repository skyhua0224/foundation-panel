//! Live touch-keyboard preference used by the Windows Settings handler.
//!
//! Windows keeps this session-scoped WNF state separately from TabletTip's
//! registry values. WM_SETTINGCHANGE does not update it. These are internal
//! APIs: resolve them at runtime and refuse unknown payloads instead of
//! reporting a registry-only update as successful.

use std::ffi::c_void;
use std::io;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::RemoteDesktop::{
    WTS_SESSION_INFOW, WTSEnumerateSessionsW, WTSFreeMemory, WTSINFOW, WTSQuerySessionInformationW,
    WTSSessionInfo,
};
use windows::core::{s, w};

use super::current_session_id;
use super::transaction::Notification;

// Well-known state used by SettingsHandlers_Keyboard::SaveUserSetting.
// Validated on Windows 11 24H2. Do not substitute a policy or invocation event.
const WNF_TKBN_DESKTOP_MODE_AUTO_IHM: u64 = 0x0F84_0539_A3BC_B035;

type Query = unsafe extern "system" fn(
    *const u64,
    *const c_void,
    *const c_void,
    *mut u32,
    *mut u32,
    *mut u32,
) -> i32;
type Publish = unsafe extern "system" fn(u64, *const c_void, *const u32, u32, *const c_void) -> i32;

#[derive(Clone, Copy)]
struct Api {
    query: Query,
    publish: Publish,
}

impl Api {
    fn load() -> io::Result<Self> {
        let unsupported = || {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "Windows touch keyboard notification API is unavailable",
            )
        };
        // ntdll is already loaded for the process lifetime; do not search for
        // or load a DLL from the application directory.
        unsafe {
            let module = GetModuleHandleW(w!("ntdll.dll")).map_err(|_| unsupported())?;
            let query =
                GetProcAddress(module, s!("NtQueryWnfStateData")).ok_or_else(unsupported)?;
            let publish =
                GetProcAddress(module, s!("RtlPublishWnfStateData")).ok_or_else(unsupported)?;
            Ok(Self {
                query: std::mem::transmute::<unsafe extern "system" fn() -> isize, Query>(query),
                publish: std::mem::transmute::<unsafe extern "system" fn() -> isize, Publish>(
                    publish,
                ),
            })
        }
    }
}

#[derive(Default)]
pub(super) struct WnfNotification {
    api: Option<Api>,
}

impl WnfNotification {
    fn api(&mut self, session_id: u32) -> io::Result<Api> {
        if current_session_id()? != session_id {
            return Err(io::Error::other(
                "touch keyboard notification belongs to a different Windows session",
            ));
        }
        if let Some(api) = self.api {
            return Ok(api);
        }
        let api = Api::load()?;
        self.api = Some(api);
        Ok(api)
    }
}

fn nt_result(status: i32, operation: &str) -> io::Result<()> {
    if status < 0 {
        Err(io::Error::other(format!(
            "touch keyboard WNF {operation} failed: NTSTATUS 0x{:08X}",
            status as u32,
        )))
    } else {
        Ok(())
    }
}

fn unsupported_payload(size: u32, stamp: u32, data: &[u8]) -> io::Error {
    let shown = size.min(data.len() as u32) as usize;
    let hex = data[..shown]
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ");
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("unexpected touch keyboard notification payload: size={size} stamp={stamp} data={hex}"),
    )
}

impl Notification for WnfNotification {
    fn current_session(&mut self) -> io::Result<u32> {
        current_session_id()
    }

    fn logon_time(&mut self, session_id: u32) -> io::Result<Option<i64>> {
        // Enumeration distinguishes an ended session from query/access errors.
        let mut sessions: *mut WTS_SESSION_INFOW = std::ptr::null_mut();
        let mut count = 0;
        unsafe { WTSEnumerateSessionsW(None, 0, 1, &mut sessions, &mut count) }
            .map_err(|e| io::Error::other(format!("enumerating Windows sessions failed: {e}")))?;
        let exists = if count == 0 {
            false
        } else if sessions.is_null() {
            return Err(io::Error::other("Windows session list is invalid"));
        } else {
            unsafe { std::slice::from_raw_parts(sessions, count as usize) }
                .iter()
                .any(|session| session.SessionId == session_id)
        };
        unsafe { WTSFreeMemory(sessions.cast()) };
        if !exists {
            return Ok(None);
        }
        let mut buffer = windows::core::PWSTR::null();
        let mut size = 0;
        unsafe {
            WTSQuerySessionInformationW(None, session_id, WTSSessionInfo, &mut buffer, &mut size)
        }
        .map_err(|e| io::Error::other(format!("querying Windows session failed: {e}")))?;
        let result = if buffer.is_null() || (size as usize) < std::mem::size_of::<WTSINFOW>() {
            Err(io::Error::other("Windows session information is invalid"))
        } else {
            let info = unsafe { std::ptr::read_unaligned(buffer.0.cast::<WTSINFOW>()) };
            if info.SessionId != session_id || info.LogonTime <= 0 {
                Err(io::Error::other(
                    "Windows session logon time is unavailable",
                ))
            } else {
                Ok(Some(info.LogonTime))
            }
        };
        unsafe { WTSFreeMemory(buffer.0.cast()) };
        result
    }

    fn read(&mut self, session_id: u32) -> io::Result<Option<u32>> {
        let api = self.api(session_id)?;
        // A buffer larger than the expected payload keeps an unexpected
        // state readable for diagnostics instead of failing with an
        // opaque buffer-too-small status.
        let mut data = [0u8; 16];
        let mut size = data.len() as u32;
        let mut stamp = 0;
        nt_result(
            unsafe {
                (api.query)(
                    &WNF_TKBN_DESKTOP_MODE_AUTO_IHM,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut stamp,
                    data.as_mut_ptr().cast::<u32>(),
                    &mut size,
                )
            },
            "query",
        )?;
        if size == 0 {
            // Windows has not published this state yet. This is normal on
            // the first query after a reboot, before the Settings handler
            // or this module has ever written a value. The transaction
            // records it as "no original runtime value".
            return Ok(None);
        }
        if size != 4 {
            return Err(unsupported_payload(size, stamp, &data));
        }
        let value = u32::from_le_bytes(data[..4].try_into().expect("size checked as 4"));
        if value > 2 {
            return Err(unsupported_payload(size, stamp, &data));
        }
        Ok(Some(value))
    }

    fn publish(&mut self, session_id: u32, value: u32) -> io::Result<()> {
        if value > 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid touch keyboard mode",
            ));
        }
        let api = self.api(session_id)?;
        nt_result(
            unsafe {
                (api.publish)(
                    WNF_TKBN_DESKTOP_MODE_AUTO_IHM,
                    std::ptr::null(),
                    &value,
                    4,
                    std::ptr::null(),
                )
            },
            "publish",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_recovery_into_a_different_windows_session() {
        let other_session = current_session_id().unwrap().wrapping_add(1);
        let mut notification = WnfNotification::default();
        assert!(notification.read(other_session).is_err());
        assert!(notification.publish(other_session, 0).is_err());
        assert!(notification.api.is_none());
    }

    #[test]
    fn reads_current_windows_logon_time() {
        let session = current_session_id().unwrap();
        assert!(
            WnfNotification::default()
                .logon_time(session)
                .unwrap()
                .unwrap()
                > 0
        );
    }
}
