//! Windows foreground-focus diagnostics for the remote text-context observer.
//!
//! This intentionally does not inject into or modify the inspected process. It
//! reports what WinEvent, Win32 GUI thread state, and UI Automation expose for
//! the currently focused window.

#[cfg(not(windows))]
fn main() {
    eprintln!("text-context-probe is only available on Windows");
}

#[cfg(windows)]
mod windows_probe {
    use std::cell::Cell;
    use std::thread;
    use std::time::Duration;

    use serde_json::{Value, json};
    use windows::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
        CoUninitialize, SAFEARRAY,
    };
    use windows::Win32::System::Ole::{
        SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
        SafeArrayUnaccessData,
    };
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
        QueryFullProcessImageNameW,
    };
    use windows::Win32::UI::Accessibility::{
        CUIAutomation, HWINEVENTHOOK, IUIAutomation, IUIAutomationCacheRequest,
        IUIAutomationElement, IUIAutomationTextEditPattern, IUIAutomationTextPattern2,
        IUIAutomationValuePattern, SetWinEventHook, UIA_BoundingRectanglePropertyId,
        UIA_ClassNamePropertyId, UIA_ControlTypePropertyId, UIA_HasKeyboardFocusPropertyId,
        UIA_IsEnabledPropertyId, UIA_IsOffscreenPropertyId, UIA_IsPasswordPropertyId,
        UIA_TextEditPatternId, UIA_TextPattern2Id, UIA_ValueIsReadOnlyPropertyId,
        UIA_ValuePatternId, UnhookWinEvent,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, EVENT_OBJECT_FOCUS, EVENT_SYSTEM_FOREGROUND, GUITHREADINFO,
        GetClassNameW, GetForegroundWindow, GetGUIThreadInfo, GetWindowRect, GetWindowTextW,
        GetWindowThreadProcessId, MSG, PM_REMOVE, PeekMessageW, TranslateMessage,
        WINEVENT_OUTOFCONTEXT,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct WireRect {
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    }

    impl From<RECT> for WireRect {
        fn from(rect: RECT) -> Self {
            Self {
                left: rect.left,
                top: rect.top,
                right: rect.right,
                bottom: rect.bottom,
            }
        }
    }

    impl WireRect {
        fn visible(self) -> bool {
            self.right > self.left && self.bottom > self.top
        }
    }

    #[derive(Clone, Debug)]
    struct EventInfo {
        event: u32,
        hwnd: HWND,
        object: i32,
        child: i32,
        thread: u32,
        time: u32,
    }

    thread_local! {
        static LAST_EVENT: Cell<Option<EventInfo>> = const { Cell::new(None) };
    }

    unsafe extern "system" fn win_event_proc(
        _hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        object: i32,
        child: i32,
        thread: u32,
        time: u32,
    ) {
        LAST_EVENT.with(|slot| {
            slot.set(Some(EventInfo {
                event,
                hwnd,
                object,
                child,
                thread,
                time,
            }))
        });
    }

    fn take_event() -> Option<EventInfo> {
        LAST_EVENT.with(Cell::take)
    }

    fn pump_messages() {
        let mut message = MSG::default();
        unsafe {
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }
    }

    struct FocusHook(HWINEVENTHOOK);

    impl Drop for FocusHook {
        fn drop(&mut self) {
            if !self.0.is_invalid() {
                let _ = unsafe { UnhookWinEvent(self.0) };
            }
        }
    }

    fn class_name(hwnd: HWND) -> Option<String> {
        if hwnd.is_invalid() {
            return None;
        }
        let mut buffer = [0u16; 256];
        let length = unsafe { GetClassNameW(hwnd, &mut buffer) }.max(0) as usize;
        (length > 0).then(|| String::from_utf16_lossy(&buffer[..length.min(buffer.len())]))
    }

    fn window_title(hwnd: HWND) -> Option<String> {
        if hwnd.is_invalid() {
            return None;
        }
        let mut buffer = [0u16; 512];
        let length = unsafe { GetWindowTextW(hwnd, &mut buffer) }.max(0) as usize;
        (length > 0).then(|| String::from_utf16_lossy(&buffer[..length.min(buffer.len())]))
    }

    fn window_rect(hwnd: HWND) -> Option<WireRect> {
        if hwnd.is_invalid() {
            return None;
        }
        let mut rect = RECT::default();
        unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
        let rect = WireRect::from(rect);
        rect.visible().then_some(rect)
    }

    fn process_image(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buffer = vec![0u16; 32768];
            let mut length = buffer.len() as u32;
            let result = QueryFullProcessImageNameW(
                handle,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(buffer.as_mut_ptr()),
                &mut length,
            );
            let _ = CloseHandle(handle);
            result.ok()?;
            Some(String::from_utf16_lossy(&buffer[..length as usize]))
        }
    }

    fn rect_json(rect: Option<WireRect>) -> Value {
        rect.map_or(Value::Null, |rect| {
            json!({
                "left": rect.left,
                "top": rect.top,
                "right": rect.right,
                "bottom": rect.bottom,
            })
        })
    }

    fn hwnd_json(hwnd: HWND) -> Value {
        if hwnd.is_invalid() {
            Value::Null
        } else {
            json!(format!("0x{:X}", hwnd.0 as usize))
        }
    }

    fn window_json(hwnd: HWND) -> Value {
        if hwnd.is_invalid() {
            return Value::Null;
        }
        let mut pid = 0u32;
        let thread = unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
        json!({
            "hwnd": hwnd_json(hwnd),
            "class": class_name(hwnd),
            "title": window_title(hwnd),
            "rect": rect_json(window_rect(hwnd)),
            "pid": pid,
            "thread": thread,
            "image": process_image(pid),
        })
    }

    unsafe fn last_bounding_rect(array: *mut SAFEARRAY) -> Option<WireRect> {
        if array.is_null() {
            return None;
        }
        let result = (|| {
            let lower = unsafe { SafeArrayGetLBound(array, 1) }.ok()?;
            let upper = unsafe { SafeArrayGetUBound(array, 1) }.ok()?;
            let count = upper.checked_sub(lower)?.checked_add(1)? as usize;
            if count < 4 || count % 4 != 0 {
                return None;
            }
            let mut raw = std::ptr::null_mut();
            unsafe { SafeArrayAccessData(array, &mut raw) }.ok()?;
            let values = unsafe { std::slice::from_raw_parts(raw.cast::<f64>(), count) };
            let rect = values.chunks_exact(4).last().and_then(|values| {
                let (left, top, width, height) = (values[0], values[1], values[2], values[3]);
                if !left.is_finite()
                    || !top.is_finite()
                    || !width.is_finite()
                    || !height.is_finite()
                {
                    return None;
                }
                Some(WireRect {
                    left: left.floor() as i32,
                    top: top.floor() as i32,
                    right: (left + width.max(1.0)).ceil() as i32,
                    bottom: (top + height.max(1.0)).ceil() as i32,
                })
            });
            let _ = unsafe { SafeArrayUnaccessData(array) };
            rect.filter(|rect| rect.visible())
        })();
        let _ = unsafe { SafeArrayDestroy(array) };
        result
    }

    fn native_caret(info: &GUITHREADINFO) -> Option<WireRect> {
        if info.hwndCaret.is_invalid() {
            return None;
        }
        unsafe {
            let mut top_left = POINT {
                x: info.rcCaret.left,
                y: info.rcCaret.top,
            };
            let mut bottom_right = POINT {
                x: info.rcCaret.right,
                y: info.rcCaret.bottom,
            };
            if !ClientToScreen(info.hwndCaret, &mut top_left).as_bool()
                || !ClientToScreen(info.hwndCaret, &mut bottom_right).as_bool()
            {
                return None;
            }
            let rect = WireRect {
                left: top_left.x,
                top: top_left.y,
                right: bottom_right.x.max(top_left.x + 1),
                bottom: bottom_right.y.max(top_left.y + 1),
            };
            rect.visible().then_some(rect)
        }
    }

    fn control_type_name(value: i32) -> &'static str {
        match value {
            50000 => "Button",
            50001 => "Calendar",
            50002 => "CheckBox",
            50003 => "ComboBox",
            50004 => "Edit",
            50005 => "Hyperlink",
            50006 => "Text",
            50007 => "Group",
            50008 => "Image",
            50009 => "ListItem",
            50010 => "List",
            50011 => "Menu",
            50012 => "MenuBar",
            50013 => "MenuItem",
            50014 => "ProgressBar",
            50015 => "RadioButton",
            50016 => "ScrollBar",
            50017 => "Slider",
            50018 => "Spinner",
            50019 => "StatusBar",
            50020 => "Tab",
            50021 => "TabItem",
            50022 => "TextBox",
            50023 => "Thumb",
            50024 => "Tree",
            50025 => "TreeItem",
            50026 => "Custom",
            50027 => "ListView",
            50028 => "TreeView",
            50029 => "Window",
            50030 => "Document",
            50031 => "Pane",
            50032 => "Header",
            50033 => "HeaderItem",
            50034 => "Table",
            50035 => "TitleBar",
            50036 => "Separator",
            50037 => "SemanticZoom",
            _ => "Unknown",
        }
    }

    fn is_native_text_class(class: &str) -> bool {
        let class = class.trim().to_ascii_lowercase();
        class == "edit"
            || class.starts_with("richedit")
            || class.starts_with("windowsforms10.edit")
            || class == "scintilla"
            || class == "mintty"
            || class == "consolewindowclass"
            || class == "cascadia_hosting_window_class"
            || class.starts_with("chrome_renderwidgethosthwnd")
            || (class.starts_with("qt") && class.ends_with("qwindowicon"))
    }

    fn ui_element_json(element: &IUIAutomationElement, inspect_caret: bool) -> Value {
        let control_type = unsafe { element.CachedControlType() }
            .ok()
            .map(|value| value.0);
        let class = unsafe { element.CachedClassName() }
            .ok()
            .map(|value| value.to_string());
        let value_pattern = unsafe {
            element.GetCachedPatternAs::<IUIAutomationValuePattern>(
                windows::Win32::UI::Accessibility::UIA_ValuePatternId,
            )
        }
        .ok();
        let text_edit_pattern = unsafe {
            element.GetCachedPatternAs::<IUIAutomationTextEditPattern>(
                windows::Win32::UI::Accessibility::UIA_TextEditPatternId,
            )
        }
        .is_ok();
        let text_pattern2 = unsafe {
            element.GetCachedPatternAs::<IUIAutomationTextPattern2>(
                windows::Win32::UI::Accessibility::UIA_TextPattern2Id,
            )
        }
        .ok();
        let (caret_active, caret_rect) = if inspect_caret {
            if let Some(ref pattern) = text_pattern2 {
                let mut active = windows::core::BOOL::default();
                if let Ok(range) = unsafe { pattern.GetCaretRange(&mut active) } {
                    let rect = unsafe { range.GetBoundingRectangles() }
                        .ok()
                        .and_then(|bounds| unsafe { last_bounding_rect(bounds) });
                    (active.as_bool(), rect)
                } else {
                    (false, None)
                }
            } else {
                (false, None)
            }
        } else {
            (false, None)
        };
        let value_read_only = value_pattern
            .as_ref()
            .and_then(|pattern| unsafe { pattern.CachedIsReadOnly() }.ok())
            .map(|value| value.as_bool());
        let is_enabled = unsafe { element.CachedIsEnabled() }
            .ok()
            .map(|value| value.as_bool());
        let has_focus = unsafe { element.CachedHasKeyboardFocus() }
            .ok()
            .map(|value| value.as_bool());
        let offscreen = unsafe { element.CachedIsOffscreen() }
            .ok()
            .map(|value| value.as_bool());
        let password = unsafe { element.CachedIsPassword() }
            .ok()
            .map(|value| value.as_bool());
        json!({
            "has_keyboard_focus": has_focus,
            "enabled": is_enabled,
            "offscreen": offscreen,
            "password": password,
            "control_type": control_type,
            "control_type_name": control_type.map(control_type_name),
            "class": class,
            "class_is_native_text_candidate": class.as_deref().is_some_and(is_native_text_class),
            "element_rect": rect_json(unsafe { element.CachedBoundingRectangle() }.ok().map(WireRect::from)),
            "value_pattern": value_pattern.is_some(),
            "value_is_read_only": value_read_only,
            "text_edit_pattern": text_edit_pattern,
            "text_pattern2": text_pattern2.is_some(),
            "caret_active": caret_active,
            "caret_rect": rect_json(caret_rect),
        })
    }

    fn build_cache(automation: &IUIAutomation) -> Option<IUIAutomationCacheRequest> {
        let request = unsafe { automation.CreateCacheRequest() }.ok()?;
        unsafe {
            let _ = request.AddProperty(UIA_HasKeyboardFocusPropertyId);
            let _ = request.AddProperty(UIA_IsEnabledPropertyId);
            let _ = request.AddProperty(UIA_IsOffscreenPropertyId);
            let _ = request.AddProperty(UIA_BoundingRectanglePropertyId);
            let _ = request.AddProperty(UIA_ControlTypePropertyId);
            let _ = request.AddProperty(UIA_ClassNamePropertyId);
            let _ = request.AddProperty(UIA_IsPasswordPropertyId);
            let _ = request.AddProperty(UIA_ValueIsReadOnlyPropertyId);
            let _ = request.AddPattern(UIA_ValuePatternId);
            let _ = request.AddPattern(UIA_TextEditPatternId);
            let _ = request.AddPattern(UIA_TextPattern2Id);
        }
        Some(request)
    }

    fn uia_snapshot(
        automation: Option<&IUIAutomation>,
        cache: Option<&IUIAutomationCacheRequest>,
        info: &GUITHREADINFO,
    ) -> Value {
        let Some((automation, cache)) = automation.zip(cache) else {
            return json!({ "available": false, "error": "UIA initialization failed" });
        };
        let focused = unsafe { automation.GetFocusedElementBuildCache(cache) }
            .ok()
            .map(|element| ui_element_json(&element, true));
        let focus_handle = if info.hwndFocus.is_invalid() {
            None
        } else {
            unsafe { automation.ElementFromHandleBuildCache(info.hwndFocus, cache) }
                .ok()
                .map(|element| ui_element_json(&element, false))
        };
        let caret_handle = if info.hwndCaret.is_invalid() {
            None
        } else {
            unsafe { automation.ElementFromHandleBuildCache(info.hwndCaret, cache) }
                .ok()
                .map(|element| ui_element_json(&element, false))
        };
        json!({
            "available": true,
            "focused_element": focused,
            "element_from_hwnd_focus": focus_handle,
            "element_from_hwnd_caret": caret_handle,
        })
    }

    fn event_json(event: Option<EventInfo>) -> Value {
        let Some(event) = event else {
            return Value::Null;
        };
        json!({
            "type": match event.event {
                EVENT_OBJECT_FOCUS => "EVENT_OBJECT_FOCUS",
                EVENT_SYSTEM_FOREGROUND => "EVENT_SYSTEM_FOREGROUND",
                _ => "other",
            },
            "value": event.event,
            "hwnd": window_json(event.hwnd),
            "object": event.object,
            "child": event.child,
            "thread": event.thread,
            "time": event.time,
        })
    }

    fn snapshot(
        automation: Option<&IUIAutomation>,
        cache: Option<&IUIAutomationCacheRequest>,
        event: Option<EventInfo>,
    ) -> Value {
        let foreground = unsafe { GetForegroundWindow() };
        let mut pid = 0u32;
        let thread = unsafe { GetWindowThreadProcessId(foreground, Some(&mut pid)) };
        let mut info = GUITHREADINFO {
            cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
            ..Default::default()
        };
        let gui_info = if thread != 0 && unsafe { GetGUIThreadInfo(thread, &mut info) }.is_ok() {
            json!({
                "thread": thread,
                "active": window_json(info.hwndActive),
                "focus": window_json(info.hwndFocus),
                "capture": window_json(info.hwndCapture),
                "caret": window_json(info.hwndCaret),
                "caret_rect": rect_json(native_caret(&info)),
            })
        } else {
            json!({ "thread": thread, "error": "GetGUIThreadInfo failed" })
        };
        let native_caret_rect = native_caret(&info);
        let focus_class = class_name(info.hwndFocus);
        let caret_class = class_name(info.hwndCaret);
        let uia = uia_snapshot(automation, cache, &info);
        let uia_focused = uia.get("focused_element");
        let uia_edit = uia_focused
            .and_then(|value| value.get("control_type_name"))
            .and_then(Value::as_str)
            == Some("Edit");
        let uia_text_edit = uia_focused
            .and_then(|value| value.get("text_edit_pattern"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let uia_writable_value = uia_focused
            .and_then(|value| value.get("value_is_read_only"))
            .and_then(Value::as_bool)
            == Some(false);
        let uia_caret = uia_focused
            .and_then(|value| value.get("caret_active"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let native_text_class = [focus_class.as_deref(), caret_class.as_deref()]
            .into_iter()
            .flatten()
            .any(is_native_text_class);
        json!({
            "foreground": window_json(foreground),
            "foreground_pid": pid,
            "gui_thread": gui_info,
            "capture_evidence": {
                "uia_edit_control": uia_edit,
                "uia_text_edit_pattern": uia_text_edit,
                "uia_writable_value": uia_writable_value,
                "uia_caret": uia_caret,
                "native_caret": native_caret_rect.is_some(),
                "native_text_class": native_text_class,
                "focus_class": focus_class,
                "caret_class": caret_class,
            },
            "uia": uia,
            "event": event_json(event),
        })
    }

    fn parse_args() -> (bool, Duration) {
        let mut once = false;
        let mut interval_ms = 250u64;
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            if arg == "--once" {
                once = true;
            } else if let Some(value) = arg.strip_prefix("--interval-ms=") {
                interval_ms = value.parse().unwrap_or(interval_ms).clamp(50, 5000);
            } else if arg == "--interval-ms" {
                if let Some(value) = args.next() {
                    interval_ms = value.parse().unwrap_or(interval_ms).clamp(50, 5000);
                }
            } else if arg == "--help" || arg == "-h" {
                println!("text-context-probe [--once] [--interval-ms N]");
                println!(
                    "Prints the foreground window, native caret, WinEvent and UIA state as JSON."
                );
                std::process::exit(0);
            }
        }
        (once, Duration::from_millis(interval_ms))
    }

    pub fn run() {
        let (once, interval) = parse_args();
        println!("Focus QQ, MSYS2, Notepad++, or another editor. Press Ctrl+C to stop.");
        let initialized = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        if !initialized {
            eprintln!("COM initialization failed");
            return;
        }
        let automation =
            unsafe { CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok() };
        let cache = automation.as_ref().and_then(build_cache);
        let hook_focus = FocusHook(unsafe {
            SetWinEventHook(
                EVENT_OBJECT_FOCUS,
                EVENT_OBJECT_FOCUS,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        });
        let hook_foreground = FocusHook(unsafe {
            SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                None,
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            )
        });
        let _hooks = (hook_focus, hook_foreground);
        let mut previous = None;
        loop {
            pump_messages();
            let current = snapshot(automation.as_ref(), cache.as_ref(), take_event());
            if previous.as_ref() != Some(&current) {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&current).unwrap_or_else(|_| current.to_string())
                );
                previous = Some(current);
            }
            if once {
                break;
            }
            thread::sleep(interval);
        }
        unsafe { CoUninitialize() };
    }
}

#[cfg(windows)]
fn main() {
    windows_probe::run();
}
