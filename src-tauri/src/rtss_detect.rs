//! Where RivaTuner Statistics Server is installed.
//!
//! The installer is 32-bit, so InstallDir is written in the 32-bit registry view
//! [dr-ms-registry-redirector-20261109] [dr-ms-alt-registry-view-20261109].
//! A running RTSS.exe, RTSSHooksLoader64.exe or EncoderServer.exe names the directory when
//! that value is missing, quoted, or points at a folder that was moved. The shared-memory
//! object is RTSSSharedMemoryV2 [dr-ms-openfilemappingw-20261109].

pub fn normalize_rtss_dir(raw: &str) -> String {
    let trimmed = raw.trim().trim_matches(|c| c == '"' || c == '\'');
    trimmed.trim().trim_end_matches(['\\', '/']).to_string()
}

const PROCESS_NAMES: &[&str] = &[
    "rtss.exe",
    "rtsshooksloader.exe",
    "rtsshooksloader64.exe",
    "encoderserver.exe",
];

pub fn install_dir_from_process_image(image: &str) -> Option<String> {
    // Windows image paths use '\'. Path::file_name only splits on the host separator,
    // so a Linux test (and a forwarded path) would miss RTSS.exe. Split on both.
    let trimmed = image.trim().trim_matches('"');
    let (parent, name) = split_path(trimmed);
    let name = name.to_ascii_lowercase();
    if !PROCESS_NAMES.contains(&name.as_str()) {
        return None;
    }
    let dir = normalize_rtss_dir(parent);
    if dir.is_empty() { None } else { Some(dir) }
}

fn split_path(path: &str) -> (&str, &str) {
    match path.rfind(['\\', '/']) {
        Some(index) => (&path[..index], &path[index + 1..]),
        None => ("", path),
    }
}

pub fn choose_rtss_install_dir(
    registry_values: &[String],
    process_images: &[String],
    mut exists: impl FnMut(&str) -> bool,
) -> Option<String> {
    for raw in registry_values {
        let dir = normalize_rtss_dir(raw);
        if !dir.is_empty() && exists(&dir) {
            return Some(dir);
        }
    }
    for image in process_images {
        if let Some(dir) = install_dir_from_process_image(image) {
            if exists(&dir) {
                return Some(dir);
            }
        }
    }
    for path in [
        r"C:\Program Files (x86)\RivaTuner Statistics Server",
        r"C:\Program Files\RivaTuner Statistics Server",
    ] {
        if exists(path) {
            return Some(path.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_and_a_trailing_slash_are_not_part_of_the_directory() {
        assert_eq!(
            normalize_rtss_dir("  \"D:\\Tools\\RTSS\\\"  "),
            r"D:\Tools\RTSS"
        );
    }

    #[test]
    fn a_running_process_names_the_directory_when_the_registry_path_is_gone() {
        let images = vec![r"D:\Tools\RTSS\RTSS.exe".to_string()];
        let chosen = choose_rtss_install_dir(&[r"C:\Missing\RTSS".to_string()], &images, |path| {
            path == r"D:\Tools\RTSS"
        });
        assert_eq!(chosen.as_deref(), Some(r"D:\Tools\RTSS"));
    }

    #[test]
    fn an_unrelated_process_is_not_an_install() {
        assert!(install_dir_from_process_image(r"C:\Windows\notepad.exe").is_none());
    }
}
