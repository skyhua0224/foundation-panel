//! The picture behind the splash: a file the host named, read once and handed to the page as a data URL, so the page needs no host call and no file access.
//! Only images, only up to a size a launch screen can afford.

use base64::Engine as _;
use std::path::Path;

pub const LIMIT_BYTES: u64 = 8 * 1024 * 1024;

fn mime_of(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

/// `data:<mime>;base64,...` for an image file within the limit, else None.
pub fn data_url(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    let path = Path::new(path);
    let mime = mime_of(path)?;
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > LIMIT_BYTES {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("launch-splash-art-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_image_file_becomes_a_data_url_with_its_type() {
        let dir = scratch("ok");
        std::fs::write(dir.join("hero.JPG"), [0xFF, 0xD8, 0xFF]).unwrap();
        std::fs::write(dir.join("poster.png"), [0x89, b'P', b'N', b'G']).unwrap();
        assert_eq!(data_url(dir.join("hero.JPG").to_str().unwrap()).as_deref(), Some("data:image/jpeg;base64,/9j/"));
        assert!(data_url(dir.join("poster.png").to_str().unwrap()).unwrap().starts_with("data:image/png;base64,"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn what_is_not_a_usable_picture_gives_none() {
        let dir = scratch("bad");
        std::fs::write(dir.join("secret.txt"), b"password").unwrap();
        std::fs::write(dir.join("empty.png"), b"").unwrap();
        assert_eq!(data_url(""), None);
        assert_eq!(data_url(dir.join("missing.png").to_str().unwrap()), None);
        assert_eq!(data_url(dir.join("secret.txt").to_str().unwrap()), None, "only images are read");
        assert_eq!(data_url(dir.join("empty.png").to_str().unwrap()), None);
        assert_eq!(data_url(dir.to_str().unwrap()), None, "a folder is no picture");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_picture_over_the_limit_is_left_out() {
        let dir = scratch("big");
        let file = std::fs::File::create(dir.join("huge.png")).unwrap();
        file.set_len(LIMIT_BYTES + 1).unwrap();
        assert_eq!(data_url(dir.join("huge.png").to_str().unwrap()), None);
        let _ = std::fs::remove_dir_all(dir);
    }
}
