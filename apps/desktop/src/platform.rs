use gpui_kit::*;
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuItem},
};

pub fn create_tray() -> Result<TrayIcon, Box<dyn std::error::Error>> {
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id("agenda", "Upcoming schedule", true, None),
        &MenuItem::with_id("open", "Open Calendar", true, None),
        &MenuItem::with_id("quit", "Quit Calendar", true, None),
    ])?;
    let mut pixels = Vec::new();
    for vertical in 0..32 {
        for horizontal in 0..32 {
            let distance = (horizontal as f32 - 15.5).powi(2) + (vertical as f32 - 15.5).powi(2);
            let inside = (30.0..180.0).contains(&distance);
            pixels.extend_from_slice(&[183, 180, 239, if inside { 255 } else { 0 }]);
        }
    }
    Ok(TrayIconBuilder::new()
        .with_tooltip("Calendar · upcoming schedule")
        .with_icon(Icon::from_rgba(pixels, 32, 32)?)
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_icon_as_template(cfg!(target_os = "macos"))
        .build()?)
}

pub fn backdrop() -> WindowBackgroundAppearance {
    WindowBackgroundAppearance::Opaque
}

pub fn agenda_bounds(context: &App) -> Bounds<Pixels> {
    #[cfg(target_os = "windows")]
    if let Some(bounds) = windows_agenda_bounds() {
        return bounds;
    }
    let display = context.primary_display().expect("Desktop display");
    let bounds = display.visible_bounds();
    #[cfg(target_os = "macos")]
    let vertical = bounds.origin.y + px(34.0);
    #[cfg(not(target_os = "macos"))]
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
