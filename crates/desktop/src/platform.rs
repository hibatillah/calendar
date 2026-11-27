use gpui_kit::*;

pub fn backdrop() -> WindowBackgroundAppearance {
    WindowBackgroundAppearance::Opaque
}

/// Brings the main window of the UI process that already runs to the front. Does nothing when
/// that process only shows the tray agenda popup, which has no title.
pub fn focus_existing_window() {
    use windows::{
        Win32::UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow,
        },
        core::HSTRING,
    };
    // SAFETY: plain Win32 calls with a valid wide string; a missing window yields an error.
    unsafe {
        if let Ok(window) = FindWindowW(None, &HSTRING::from(crate::WINDOW_TITLE)) {
            if IsIconic(window).as_bool() {
                let _ = ShowWindow(window, SW_RESTORE);
            }
            let _ = SetForegroundWindow(window);
        }
    }
}

pub fn agenda_bounds(context: &App) -> Bounds<Pixels> {
    #[cfg(target_os = "windows")]
    if let Some(bounds) = windows_agenda_bounds() {
        return bounds;
    }
    let display = context.primary_display().expect("Desktop display");
    let bounds = display.visible_bounds();
    let vertical = bounds.origin.y + bounds.size.height - px(540.0);
    Bounds {
        origin: point(bounds.origin.x + bounds.size.width - px(400.0), vertical),
        size: size(px(380.0), px(470.0)),
    }
}

#[cfg(target_os = "windows")]
fn windows_agenda_bounds() -> Option<Bounds<Pixels>> {
    use windows::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint},
        UI::{
            HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI},
            WindowsAndMessaging::GetCursorPos,
        },
    };
    // The tray API reports physical coordinates; GPUI window bounds use logical pixels.
    unsafe {
        let mut cursor = POINT::default();
        GetCursorPos(&mut cursor).ok()?;
        let monitor = MonitorFromPoint(cursor, MONITOR_DEFAULTTONEAREST);
        let mut information = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut information).as_bool() {
            return None;
        }
        let mut horizontal_dpi = 96;
        let mut vertical_dpi = 96;
        GetDpiForMonitor(
            monitor,
            MDT_EFFECTIVE_DPI,
            &mut horizontal_dpi,
            &mut vertical_dpi,
        )
        .ok()?;
        let scale = horizontal_dpi as f32 / 96.0;
        let work = information.rcWork;
        let width = 380.0_f32.min((work.right - work.left) as f32 / scale);
        let height = 470.0_f32.min((work.bottom - work.top) as f32 / scale);
        let horizontal = (cursor.x as f32 / scale - width / 2.0)
            .clamp(work.left as f32 / scale, work.right as f32 / scale - width);
        let vertical = (cursor.y as f32 / scale - height - 8.0)
            .clamp(work.top as f32 / scale, work.bottom as f32 / scale - height);
        Some(Bounds::new(
            point(px(horizontal), px(vertical)),
            size(px(width), px(height)),
        ))
    }
}
