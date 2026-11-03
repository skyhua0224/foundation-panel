//! The cover of a panel while WebView2 settles after a change of scale: a child window of the panel's window that lies over the WebView and shows either the
//! panel's surface colour (adaptive) or the last frame of the page, scaled to the new size (snapshot). The page keeps running underneath, tells the host when its
//! layout has matched the window for a while (`panel_dpi_settled`) and the cover fades out over it.
//! The frame comes from the WebView's own CapturePreview into memory (PNG) and is never written anywhere:
//! https://learn.microsoft.com/microsoft-edge/webview2/reference/win32/icorewebview2#capturepreview
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, HALFTONE, HBITMAP, HGDIOBJ, PAINTSTRUCT, SRCCOPY,
    SelectObject, SetStretchBltMode, StretchBlt,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClientRect, GetWindowLongPtrW, HWND_TOP, LWA_ALPHA, RegisterClassExW, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    SetLayeredWindowAttributes, SetWindowLongPtrW, SetWindowPos, WM_ERASEBKGND, WM_NCDESTROY, WM_NCHITTEST, WM_PAINT, WNDCLASSEXW, WS_CHILD, WS_CLIPSIBLINGS, WS_EX_LAYERED,
    WS_VISIBLE,
};
use windows::core::{PCWSTR, w};

/// How the page is covered while WebView2 settles after a change of scale (SUNSHINE_GUI_WEBVIEW_DPI_COVER).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Mode {
    /// shown throughout
    Off,
    /// the page is hidden (SetIsVisible) for a fixed time
    Fixed,
    /// a cover in the surface colour until the page's layout matches the window, at most the cap
    Adaptive,
    /// the last frame of the page, scaled to the new size, until the page's layout matches the window, at most the cap
    Snapshot,
}

pub(super) fn mode_of(value: Option<&str>) -> Mode {
    match value.map(str::to_ascii_lowercase).as_deref() {
        Some("off") => Mode::Off,
        Some("adaptive") => Mode::Adaptive,
        Some("snapshot") => Mode::Snapshot,
        _ => Mode::Fixed,
    }
}

pub(super) fn mode() -> Mode {
    mode_of(std::env::var("SUNSHINE_GUI_WEBVIEW_DPI_COVER").ok().as_deref())
}

/// The longest a cover stays (the wrong layout ended 0.26 to 0.45 s after the change of scale in every measured run).
pub(super) const CAP_MS: u32 = 480;
/// The fade from the cover to the page.
pub(super) const FADE_MS: u128 = 120;

/// A decoded frame, bottom-up BGRA is not needed: CreateBitmap takes top-down 32-bit pixels.
pub(super) struct Frame {
    pub width: i32,
    pub height: i32,
    pub bgra: Vec<u8>,
}

struct Face {
    color: COLORREF,
    bitmap: Option<HBITMAP>,
    size: (i32, i32),
}

static REGISTERED: AtomicBool = AtomicBool::new(false);

fn class_name() -> PCWSTR {
    w!("SunshinePanelDpiCover")
}

fn ensure_class() {
    if REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    let instance = unsafe { windows::Win32::System::LibraryLoader::GetModuleHandleW(None) }.map(|module| windows::Win32::Foundation::HINSTANCE(module.0)).unwrap_or_default();
    let class = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(cover_proc),
        hInstance: instance,
        lpszClassName: class_name(),
        ..Default::default()
    };
    unsafe { RegisterClassExW(&class) };
}

unsafe extern "system" fn cover_proc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match message {
        WM_ERASEBKGND => LRESULT(1),
        // input goes to the page underneath
        WM_NCHITTEST => LRESULT(-1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let hdc = unsafe { BeginPaint(hwnd, &mut paint) };
            let face = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) } as *const Face;
            let mut client = RECT::default();
            let _ = unsafe { GetClientRect(hwnd, &mut client) };
            if let Some(face) = unsafe { face.as_ref() } {
                unsafe {
                    let brush = CreateSolidBrush(face.color);
                    FillRect(hdc, &client, brush);
                    let _ = DeleteObject(HGDIOBJ(brush.0));
                    if let Some(bitmap) = face.bitmap {
                        let memory = CreateCompatibleDC(Some(hdc));
                        let previous = SelectObject(memory, HGDIOBJ(bitmap.0));
                        SetStretchBltMode(hdc, HALFTONE);
                        let _ = StretchBlt(hdc, 0, 0, client.right, client.bottom, Some(memory), 0, 0, face.size.0, face.size.1, SRCCOPY);
                        SelectObject(memory, previous);
                        let _ = DeleteDC(memory);
                    }
                }
            }
            let _ = unsafe { EndPaint(hwnd, &paint) };
            LRESULT(0)
        }
        WM_NCDESTROY => {
            let face = unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) } as *mut Face;
            if !face.is_null() {
                let face = unsafe { Box::from_raw(face) };
                if let Some(bitmap) = face.bitmap {
                    let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
                }
            }
            unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

/// The cover over the client rectangle of `parent`, in `color` with `frame` stretched over it when there is one. Layered, so that it can fade.
pub(super) fn create(parent: HWND, color: (u8, u8, u8), frame: Option<&Frame>) -> Option<HWND> {
    ensure_class();
    let mut client = RECT::default();
    unsafe { GetClientRect(parent, &mut client) }.ok()?;
    let bitmap = frame.map(|frame| unsafe { CreateBitmap(frame.width, frame.height, 1, 32, Some(frame.bgra.as_ptr().cast())) });
    let face = Box::new(Face {
        color: COLORREF(u32::from(color.0) | (u32::from(color.1) << 8) | (u32::from(color.2) << 16)),
        bitmap: bitmap.filter(|bitmap| !bitmap.is_invalid()),
        size: frame.map_or((0, 0), |frame| (frame.width, frame.height)),
    });
    let face = Box::into_raw(face);
    let created = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED,
            class_name(),
            PCWSTR::null(),
            WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
            0,
            0,
            client.right.max(1),
            client.bottom.max(1),
            Some(parent),
            None,
            None,
            None,
        )
    };
    let Ok(hwnd) = created else {
        let face = unsafe { Box::from_raw(face) };
        if let Some(bitmap) = face.bitmap {
            let _ = unsafe { DeleteObject(HGDIOBJ(bitmap.0)) };
        }
        return None;
    };
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, face as isize);
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
        let _ = SetWindowPos(hwnd, Some(HWND_TOP), 0, 0, client.right.max(1), client.bottom.max(1), SWP_NOACTIVATE | SWP_SHOWWINDOW);
    }
    Some(hwnd)
}

/// The cover follows the client rectangle of its window.
pub(super) fn fit(cover: HWND, parent: HWND) {
    let mut client = RECT::default();
    if unsafe { GetClientRect(parent, &mut client) }.is_ok() && client.right > 0 && client.bottom > 0 {
        let _ = unsafe { SetWindowPos(cover, Some(HWND_TOP), 0, 0, client.right, client.bottom, SWP_NOACTIVATE) };
    }
}

/// 255 is opaque.
pub(super) fn set_opacity(cover: HWND, alpha: u8) {
    let _ = unsafe { SetLayeredWindowAttributes(cover, COLORREF(0), alpha, LWA_ALPHA) };
}

pub(super) fn destroy(cover: HWND) {
    let _ = unsafe { DestroyWindow(cover) };
}

/// The opacity `elapsed_ms` into a fade of `FADE_MS` (255 at the start, 0 at the end).
pub(super) fn fade_alpha(elapsed_ms: u128) -> u8 {
    let left = FADE_MS.saturating_sub(elapsed_ms);
    ((left * 255) / FADE_MS) as u8
}

/// The frame the WebView shows now, decoded to pixels on a thread of its own: `done` gets it (or none) on that thread.
pub(super) fn capture<F>(controller: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Controller, done: F) -> windows::core::Result<()>
where
    F: FnOnce(Option<Frame>) + Send + 'static,
{
    use webview2_com::CapturePreviewCompletedHandler;
    use webview2_com::Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG;
    use windows::Win32::System::Com::StructuredStorage::{CreateStreamOnHGlobal, GetHGlobalFromStream};
    use windows::Win32::System::Com::{STATFLAG_NONAME, STATSTG};
    let core = unsafe { controller.CoreWebView2() }?;
    let stream = unsafe { CreateStreamOnHGlobal(Default::default(), true) }?;
    let reader = stream.clone();
    let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
        let bytes = result.ok().and_then(|()| unsafe {
            let hglobal = GetHGlobalFromStream(&reader).ok()?;
            let mut stat = STATSTG::default();
            reader.Stat(&mut stat, STATFLAG_NONAME).ok()?;
            let length = stat.cbSize as usize;
            let pointer = windows::Win32::System::Memory::GlobalLock(hglobal);
            if pointer.is_null() {
                return None;
            }
            let bytes = std::slice::from_raw_parts(pointer.cast::<u8>(), length).to_vec();
            let _ = windows::Win32::System::Memory::GlobalUnlock(hglobal);
            Some(bytes)
        });
        std::thread::spawn(move || done(bytes.and_then(|bytes| decode(&bytes))));
        Ok(())
    }));
    unsafe { core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &stream, &handler) }
}

fn decode(png: &[u8]) -> Option<Frame> {
    let image = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    let mut bgra = image.into_raw();
    for pixel in bgra.chunks_exact_mut(4) {
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    Some(Frame { width: width as i32, height: height as i32, bgra })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_comes_from_the_switch_and_fixed_is_the_default() {
        assert_eq!(mode_of(None), Mode::Fixed);
        assert_eq!(mode_of(Some("")), Mode::Fixed);
        assert_eq!(mode_of(Some("OFF")), Mode::Off);
        assert_eq!(mode_of(Some("adaptive")), Mode::Adaptive);
        assert_eq!(mode_of(Some("Snapshot")), Mode::Snapshot);
    }

    #[test]
    fn a_fade_goes_from_opaque_to_clear_over_its_time() {
        assert_eq!(fade_alpha(0), 255);
        assert!(fade_alpha(FADE_MS / 2) > 120 && fade_alpha(FADE_MS / 2) < 135);
        assert_eq!(fade_alpha(FADE_MS), 0);
        assert_eq!(fade_alpha(FADE_MS * 3), 0);
    }

    #[test]
    fn a_png_becomes_bgra_pixels() {
        let mut png = Vec::new();
        let source = image::RgbaImage::from_pixel(2, 1, image::Rgba([10, 20, 30, 200]));
        image::DynamicImage::ImageRgba8(source).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let frame = decode(&png).unwrap();
        assert_eq!((frame.width, frame.height), (2, 1));
        assert_eq!(&frame.bgra[..4], &[30, 20, 10, 255]);
        assert!(decode(b"not a png").is_none());
    }
}
