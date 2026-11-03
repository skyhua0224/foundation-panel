//! Mirror酱 CDK 的操作系统安全存储。
//!
//! Windows：Credential Manager 的通用凭据（CredWriteW/CredReadW/CredDeleteW，
//! CRED_TYPE_GENERIC、CRED_PERSIST_LOCAL_MACHINE，blob 按 CredUI 约定存 UTF-16LE；
//! Microsoft Learn 文档见 docs/authority/source-index.tsv 的
//! panel-ms-credwritew-20261103、panel-ms-credreadw-20261103、
//! panel-ms-creddeletew-20261103、panel-ms-credentialw-20261103）。
//! 面板目前只有 Windows 构建（wmi 依赖仅支持 Windows），其余平台的构建会得到
//! 明确的失败而不是退化到文件存储；若未来增加 macOS 构建，对应实现是
//! Keychain 的 generic password（SecItemAdd/SecItemCopyMatching/SecItemDelete）。
//!
//! CDK 绝不进入文件、日志、崩溃报告或 IPC 回复：`update_cdk_present` 只回答
//! 布尔值，`read` 只在进程内供 HTTP 请求拼 URL 使用。错误文本只含错误码，
//! 不含凭据内容。

/// 凭据的 UserName：在凭据管理器 UI 里标识归属（产品前缀是 Microsoft 文档
/// 建议的 TargetName 命名方式，这里同样加前缀）。
const CREDENTIAL_USER_NAME: &str = "foundation-sunshine-panel";
/// CDK 是短许可密钥：拒绝空值、超长值与控制字符。
const CDK_MAX_CHARS: usize = 128;

#[cfg(target_os = "windows")]
mod windows_store {
    use windows::Win32::Security::Credentials::*;
    use windows::core::{PCWSTR, PWSTR};

    use super::CREDENTIAL_USER_NAME;

    /// ERROR_NOT_FOUND：目标凭据不存在（CredReadW/CredDeleteW 文档列出的状态码）。
    const ERROR_NOT_FOUND: u32 = 1168;

    fn to_wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }

    /// windows 绑把 GetLastError 包成 HRESULT_FROM_WIN32；取回低 16 位的
    /// Win32 错误码用于展示（不含任何凭据内容）。
    fn win32_code_of(error: &windows::core::Error) -> u32 {
        let raw = error.code().0 as u32;
        if raw & 0xFFFF_0000 == 0x8007_0000 {
            raw & 0xFFFF
        } else {
            raw
        }
    }

    fn is_not_found(error: &windows::core::Error) -> bool {
        win32_code_of(error) == ERROR_NOT_FOUND
    }

    /// 写入。同一 TargetName+Type 的凭据存在即整体替换（CredWriteW 语义）；
    /// blob 用 UTF-16LE 不带结束符：凭据管理器 UI 按 UTF-16 展示通用凭据。
    pub(super) fn set(target: &str, cdk: &str) -> Result<(), String> {
        let mut target_wide = to_wide(target);
        let mut user_name_wide = to_wide(CREDENTIAL_USER_NAME);
        let blob: Vec<u8> = cdk
            .encode_utf16()
            .flat_map(|unit| unit.to_le_bytes())
            .collect();
        let mut record = CREDENTIALW::default();
        record.Type = CRED_TYPE_GENERIC;
        record.TargetName = PWSTR(target_wide.as_mut_ptr());
        record.CredentialBlobSize = blob.len() as u32;
        record.CredentialBlob = blob.as_ptr() as *mut u8;
        record.Persist = CRED_PERSIST_LOCAL_MACHINE;
        record.UserName = PWSTR(user_name_wide.as_mut_ptr());
        unsafe { CredWriteW(&record, 0) }
            .map_err(|error| format!("写入系统凭据存储失败（错误 {}）", win32_code_of(&error)))
    }

    /// 读取。`Ok(None)` 表示没有存储过；错误文本不含 CDK。
    pub(super) fn read(target: &str) -> Result<Option<String>, String> {
        let target_wide = to_wide(target);
        let mut credential_ptr: *mut CREDENTIALW = std::ptr::null_mut();
        let result = unsafe {
            CredReadW(
                PCWSTR(target_wide.as_ptr()),
                CRED_TYPE_GENERIC,
                None,
                &mut credential_ptr,
            )
        };
        match result {
            Ok(()) => {}
            Err(error) if is_not_found(&error) => return Ok(None),
            Err(error) => {
                return Err(format!(
                    "读取系统凭据存储失败（错误 {}）",
                    win32_code_of(&error)
                ));
            }
        }
        // 单块缓冲（含内部指针）必须用 CredFree 释放；解码要在释放前完成。
        unsafe {
            let record = &*credential_ptr;
            let size = record.CredentialBlobSize as usize;
            let bytes = if record.CredentialBlob.is_null() || size == 0 {
                &[][..]
            } else {
                std::slice::from_raw_parts(record.CredentialBlob, size)
            };
            let decoded = decode_blob(bytes);
            CredFree(credential_ptr as *const core::ffi::c_void);
            decoded.map(Some)
        }
    }

    fn decode_blob(bytes: &[u8]) -> Result<String, String> {
        if bytes.is_empty() {
            return Err("存储的 CDK 为空".to_string());
        }
        if bytes.len() % 2 != 0 {
            return Err("存储的 CDK 编码无效".to_string());
        }
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let text = String::from_utf16_lossy(&units);
        if text.is_empty() {
            return Err("存储的 CDK 为空".to_string());
        }
        Ok(text)
    }

    /// 删除；目标不存在视为成功（幂等）。
    pub(super) fn clear(target: &str) -> Result<(), String> {
        let target_wide = to_wide(target);
        match unsafe { CredDeleteW(PCWSTR(target_wide.as_ptr()), CRED_TYPE_GENERIC, None) } {
            Ok(()) => Ok(()),
            Err(error) if is_not_found(&error) => Ok(()),
            Err(error) => Err(format!(
                "删除系统凭据存储失败（错误 {}）",
                win32_code_of(&error)
            )),
        }
    }
}

#[cfg(target_os = "windows")]
use windows_store as store;

#[cfg(not(target_os = "windows"))]
mod store {
    pub(super) fn set(_target: &str, _cdk: &str) -> Result<(), String> {
        Err("系统安全存储仅在 Windows 构建中可用".to_string())
    }
    pub(super) fn read(_target: &str) -> Result<Option<String>, String> {
        Err("系统安全存储仅在 Windows 构建中可用".to_string())
    }
    pub(super) fn clear(_target: &str) -> Result<(), String> {
        Err("系统安全存储仅在 Windows 构建中可用".to_string())
    }
}

/// 修剪并校验用户输入的 CDK：非空、至多 128 字符、无控制字符。
pub(crate) fn validate_cdk(raw: &str) -> Result<String, String> {
    let cdk = raw.trim();
    if cdk.is_empty() {
        return Err("CDK 不能为空".to_string());
    }
    if cdk.chars().count() > CDK_MAX_CHARS {
        return Err("CDK 过长".to_string());
    }
    if cdk.chars().any(char::is_control) {
        return Err("CDK 含有无效字符".to_string());
    }
    Ok(cdk.to_string())
}

/// 写入操作系统安全存储。
pub(crate) fn set(target: &str, cdk: &str) -> Result<(), String> {
    store::set(target, cdk)
}

/// 进程内读取（仅用于拼请求 URL；不跨 IPC）。
pub(crate) fn read(target: &str) -> Result<Option<String>, String> {
    store::read(target)
}

/// 删除；幂等。
pub(crate) fn clear(target: &str) -> Result<(), String> {
    store::clear(target)
}

/// `update_cdk_present` 的实现：只回答布尔值，失败按不存在处理（不把错误抛给渲染层）。
pub(crate) fn present(target: &str) -> bool {
    matches!(read(target), Ok(Some(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_cdk_trims_and_rejects_bad_input() {
        assert_eq!(validate_cdk("  AB-12  ").as_deref(), Ok("AB-12"));
        assert!(validate_cdk("   ").is_err());
        assert!(validate_cdk("").is_err());
        assert!(validate_cdk(&"x".repeat(129)).is_err());
        assert!(validate_cdk(&"x".repeat(128)).is_ok());
        assert!(validate_cdk("a\nb").is_err());
        assert!(validate_cdk("a\0b").is_err());
    }

    #[cfg(target_os = "windows")]
    mod windows_roundtrip {
        use super::*;

        const TEST_TARGET: &str = "foundation-sunshine-panel/mirrorchyan-cdk-test";

        #[test]
        fn set_present_read_clear_roundtrip_in_credential_manager() {
            let _ = clear(TEST_TARGET);
            set(TEST_TARGET, "test-cdk-roundtrip").expect("CredWriteW succeeds");
            assert!(present(TEST_TARGET));
            assert_eq!(
                read(TEST_TARGET).expect("CredReadW succeeds"),
                Some("test-cdk-roundtrip".to_string())
            );
            // 覆盖写：同一个 TargetName+Type 整体替换。
            set(TEST_TARGET, "test-cdk-2").expect("CredWriteW replaces");
            assert_eq!(
                read(TEST_TARGET).expect("CredReadW succeeds"),
                Some("test-cdk-2".to_string())
            );
            clear(TEST_TARGET).expect("CredDeleteW succeeds");
            assert!(!present(TEST_TARGET));
            // 幂等：再次删除不报错。
            clear(TEST_TARGET).expect("CredDeleteW is idempotent");
        }
    }
}
