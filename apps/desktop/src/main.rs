#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(all(test, feature = "ui-tests"))]
extern crate gpui_kit as gpui;
mod app;
mod platform;
mod service;
use app::CalendarApp;
use gpui_kit::{
    component::{Theme, ThemeMode, TitleBar},
    prelude::*,
    *,
};

struct DesktopState {
    _tray: Option<tray_icon::TrayIcon>,
    main: Option<WindowHandle<base::Root>>,
    popup: Option<WindowHandle<base::Root>>,
    view: Option<Entity<CalendarApp>>,
}
impl Global for DesktopState {}

fn main() {
    let _logging = calendar_telemetry::init("desktop");
    gpui_kit::application()
        .with_assets(assets::AllAssets)
        .run(|context| {
            gpui_kit::init(context);
            context
                .text_system()
                .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
                    "../../../assets/fonts/Inter.ttf"
                ))])
                .expect("Load bundled Inter font");
            Theme::change(ThemeMode::Dark, None, context);
            Theme::update(context, |theme| {
                theme.background = rgb(0x222222).into();
                theme.foreground = rgb(0xe9e9e9).into();
                theme.font_family = "Inter".into();
                theme.title_bar = rgb(0x191919).into();
                theme.title_bar_border = transparent_black();
            });
            let tray = platform::create_tray()
                .map_err(|error| tracing::warn!(%error, "Tray unavailable; close will exit"))
                .ok();
            context.set_global(DesktopState {
                _tray: tray,
                main: None,
                popup: None,
                view: None,
            });
            context
                .on_window_closed(|context, _| {
                    if context.windows().is_empty()
                        && context.global::<DesktopState>()._tray.is_none()
                    {
                        context.quit();
                    }
                })
                .detach();
            open_main(context);
            context
                .spawn(async |context| {
                    loop {
                        context
                            .background_executor()
                            .timer(std::time::Duration::from_millis(80))
                            .await;
                        context.update(|context| {
                            while let Ok(event) = tray_icon::TrayIconEvent::receiver().try_recv() {
                                if let tray_icon::TrayIconEvent::Click {
                                    button: tray_icon::MouseButton::Left,
                                    button_state: tray_icon::MouseButtonState::Up,
                                    ..
                                } = event
                                {
                                    open_agenda(context);
                                }
                            }
                            while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv()
                            {
                                match event.id.0.as_str() {
                                    "open" => open_main(context),
                                    "agenda" => open_agenda(context),
                                    "quit" => {
                                        tracing::info!(action = "quit_requested");
                                        context.quit();
                                    }
                                    _ => (),
                                }
                            }
                        });
                    }
                })
                .detach();
            context.activate(true);
        });
}

pub fn open_main(context: &mut App) {
    if let Some(handle) = context.global::<DesktopState>().main
        && handle
            .update(context, |_, window, _| window.activate_window())
            .is_ok()
    {
        context.activate(true);
        return;
    }
    let bounds = Bounds::centered(None, size(px(1050.0), px(680.0)), context);
    let handle = context
        .open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(1050.0), px(680.0))),
                window_background: platform::backdrop(),
                titlebar: Some(TitlebarOptions {
                    title: Some("Calendar".into()),
                    ..TitleBar::title_bar_options()
                }),
                app_id: Some("app.calendar.desktop".into()),
                ..TitleBar::window_options()
            },
            |window, context| {
                window.on_window_should_close(context, |_, context| {
                    if context.global::<DesktopState>()._tray.is_some() {
                        tracing::info!(action = "hidden_to_tray");
                        context.hide();
                        false
                    } else {
                        true
                    }
                });
                let view = context.new(|context| CalendarApp::new(window, context, false));
                context.global_mut::<DesktopState>().view = Some(view.clone());
                context
                    .new(|context| base::Root::new(view, window, context).bg(transparent_black()))
            },
        )
        .expect("Open Calendar window");
    context.global_mut::<DesktopState>().main = Some(handle);
    tracing::info!(action = "main_window_opened");
}

pub fn show_event(event: calendar_core::CalendarEvent, context: &mut App) {
    open_main(context);
    let handle = context.global::<DesktopState>().main;
    let view = context.global::<DesktopState>().view.clone();
    if let (Some(handle), Some(view)) = (handle, view) {
        let _ = handle.update(context, |_, window, context| {
            view.update(context, |app, context| {
                app.open_event(event, window, context)
            })
        });
    }
}

fn open_agenda(context: &mut App) {
    if let Some(handle) = context.global::<DesktopState>().popup
        && handle
            .update(context, |_, window, _| window.remove_window())
            .is_ok()
    {
        context.global_mut::<DesktopState>().popup = None;
        return;
    }
    let bounds = platform::agenda_bounds(context);
    let handle = context.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_background: platform::backdrop(),
            titlebar: None,
            is_resizable: false,
            kind: WindowKind::PopUp,
            focus: true,
            ..Default::default()
        },
        |window, context| {
            let view = context.new(|context| CalendarApp::new(window, context, true));
            context.new(|context| base::Root::new(view, window, context).bg(transparent_black()))
        },
    );
    match handle {
        Ok(handle) => {
            context.global_mut::<DesktopState>().popup = Some(handle);
            tracing::info!(action = "tray_agenda_opened");
        }
        Err(error) => tracing::error!(%error, "Could not open tray agenda"),
    }
}
