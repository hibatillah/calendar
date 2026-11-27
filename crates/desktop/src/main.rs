#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(all(test, feature = "ui-tests"))]
extern crate gpui_kit as gpui;
mod app;
mod platform;
mod service;
use app::{CalendarApp, Preferences};
use calendar_sync::{
    instance::{self, UI_INSTANCE},
    logging,
};
use gpui_kit::{
    component::{Theme, ThemeMode, TitleBar},
    prelude::*,
    *,
};
use std::time::Instant;

pub const WINDOW_TITLE: &str = "Calendar";

/// What the tray process (or the user) asked this process to show first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Launch {
    Calendar,
    Agenda,
    Settings,
}

impl Launch {
    fn from_arguments(mut arguments: impl Iterator<Item = String>) -> Self {
        match arguments.next().as_deref() {
            Some("--agenda") => Self::Agenda,
            Some("--settings") => Self::Settings,
            _ => Self::Calendar,
        }
    }
}

struct DesktopState {
    main: Option<WindowHandle<base::Root>>,
    view: Option<Entity<CalendarApp>>,
    handle: service::Handle,
    model: Entity<service::Model>,
}

impl Global for DesktopState {}

fn main() {
    let started = Instant::now();
    // A second UI process only brings the first one forward; closing the last window ends a process.
    let _instance = match instance::acquire(UI_INSTANCE) {
        Ok(instance) => instance,
        Err(_) => return platform::focus_existing_window(),
    };
    logging::init("desktop");
    let launch = Launch::from_arguments(std::env::args().skip(1));
    let (handle, updates) = service::start();
    gpui_kit::application()
        .with_assets(assets::AllAssets)
        .run(move |context| {
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
            let model = context.new(|_| service::Model::default());
            let feed = model.clone();
            context
                .spawn(async move |context| {
                    while let Ok(update) = updates.recv().await {
                        feed.update(context, |model, context| {
                            model.apply(update);
                            context.notify();
                        });
                    }
                })
                .detach();
            context.set_global(DesktopState {
                main: None,
                view: None,
                handle,
                model,
            });
            context
                .on_window_closed(|context, _| {
                    if context.windows().is_empty() {
                        tracing::info!(action = "last_window_closed");
                        context.quit();
                    }
                })
                .detach();
            match launch {
                Launch::Agenda => open_agenda(context),
                Launch::Calendar | Launch::Settings => open_main(context),
            }
            if launch == Launch::Settings
                && let Some(view) = context.global::<DesktopState>().view.clone()
            {
                view.update(context, |app, context| app.open_settings(context));
            }
            context.activate(true);
            tracing::info!(
                action = "ui_ready",
                elapsed_ms = started.elapsed().as_millis() as u64
            );
        });
}

pub fn open_main(context: &mut App) {
    if let Some(handle) = context.global::<DesktopState>().main
        && handle
            .update(context, |_, window, _| window.activate_window())
            .is_ok()
    {
        context.activate(true);
        context.global::<DesktopState>().handle.sync_now();
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
                    title: Some(WINDOW_TITLE.into()),
                    ..TitleBar::title_bar_options()
                }),
                app_id: Some("app.calendar.desktop".into()),
                ..TitleBar::window_options()
            },
            |window, context| {
                let (model, handle) = {
                    let state = context.global::<DesktopState>();
                    (state.model.clone(), state.handle.clone())
                };
                let view = context.new(|context| {
                    CalendarApp::new(
                        window,
                        context,
                        false,
                        model,
                        handle,
                        Preferences::load_from_data_directory(),
                    )
                });
                context.global_mut::<DesktopState>().view = Some(view.clone());
                context
                    .new(|context| base::Root::new(view, window, context).bg(transparent_black()))
            },
        )
        .expect("Open Calendar window");
    context.global_mut::<DesktopState>().main = Some(handle);
    context.global::<DesktopState>().handle.sync_now();
    tracing::info!(action = "main_window_opened");
}

pub fn show_event(event: calendar_core::CalendarEvent, context: &mut App) {
    open_main(context);
    let handle = context.global::<DesktopState>().main;
    let view = context.global::<DesktopState>().view.clone();
    if let (Some(handle), Some(view)) = (handle, view) {
        let _ = handle.update(context, |_, _, context| {
            view.update(context, |app, context| app.open_event(event, context))
        });
    }
}

/// The tray's "coming up" popup. It removes itself when it loses focus, so this process ends then.
fn open_agenda(context: &mut App) {
    let (model, service) = {
        let state = context.global::<DesktopState>();
        (state.model.clone(), state.handle.clone())
    };
    service.sync_now();
    let bounds = platform::agenda_bounds(context);
    let opened = context.open_window(
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
            let view = context.new(|context| {
                CalendarApp::new(
                    window,
                    context,
                    true,
                    model,
                    service,
                    Preferences::load_from_data_directory(),
                )
            });
            context.new(|context| base::Root::new(view, window, context).bg(transparent_black()))
        },
    );
    match opened {
        Ok(_) => tracing::info!(action = "tray_agenda_opened"),
        Err(error) => tracing::error!(%error, "Could not open tray agenda"),
    }
}

#[cfg(test)]
mod tests {
    use super::Launch;

    fn launch(arguments: &[&str]) -> Launch {
        Launch::from_arguments(arguments.iter().map(|argument| argument.to_string()))
    }

    #[test]
    fn arguments_choose_the_first_window() {
        assert_eq!(launch(&[]), Launch::Calendar);
        assert_eq!(launch(&["--agenda"]), Launch::Agenda);
        assert_eq!(launch(&["--settings"]), Launch::Settings);
        assert_eq!(launch(&["--unknown"]), Launch::Calendar);
    }
}
