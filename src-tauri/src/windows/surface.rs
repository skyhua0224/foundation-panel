//! The colour a panel window shows before its first frame (WebView2 DefaultBackgroundColor
//! and the native window background). The renderer reports its theme surface whenever it
//! changes; the last one is kept so the next start opens on the same colour, never on
//! white or black.
//! https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2controller2#get_defaultbackgroundcolor

use std::path::PathBuf;
use std::sync::Mutex;
use tauri::window::Color;

/// The shell's dark boot colour (sunshine-frame.html), until a theme has been reported.
const FIRST_RUN: (u8, u8, u8) = (0x13, 0x13, 0x18);

static LAST_STORED: Mutex<Option<(u8, u8, u8)>> = Mutex::new(None);

fn path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|dir| dir.join("Sunshine GUI").join("panel-surface"))
}

fn parse(text: &str) -> Option<(u8, u8, u8)> {
    let mut parts = text.trim().split(',').map(|part| part.trim().parse::<u8>());
    let color = (parts.next()?.ok()?, parts.next()?.ok()?, parts.next()?.ok()?);
    parts.next().is_none().then_some(color)
}

pub fn startup_color() -> Color {
    let (r, g, b) = path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| parse(&text))
        .unwrap_or(FIRST_RUN);
    Color(r, g, b, 255)
}

/// The surface colour now: the last one the renderer reported, or the one the window started with.
pub fn current() -> (u8, u8, u8) {
    LAST_STORED.lock().unwrap().unwrap_or_else(|| {
        let Color(r, g, b, _) = startup_color();
        (r, g, b)
    })
}

/// Remembers the surface for the next start (written only when it changed).
pub fn store(color: (u8, u8, u8)) {
    let mut last = LAST_STORED.lock().unwrap();
    if *last == Some(color) {
        return;
    }
    *last = Some(color);
    if let Some(path) = path() {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, format!("{},{},{}", color.0, color.1, color.2));
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn reads_three_bytes_and_nothing_else() {
        assert_eq!(parse("45, 38,40\n"), Some((45, 38, 40)));
        assert_eq!(parse("45,38"), None);
        assert_eq!(parse("45,38,40,1"), None);
        assert_eq!(parse("45,38,256"), None);
        assert_eq!(parse(""), None);
    }
}
