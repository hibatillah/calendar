//! The always-running tray process: tray icon, menu and the idle sync poll. No window toolkit and
//! no async runtime, so its idle memory stays small. The UI is a separate process that this one
//! starts on demand; while it runs it owns syncing and this process only re-reads the cache.
//!
//! Threads: the main thread (Win32 message loop, tray icon) and one worker thread (the
//! `calendar-sync` scheduler, one wake every five minutes plus "Sync now"). No timers.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod summary;

use calendar_google::{Config, Google};
use calendar_sync::{
    Store, Syncer, data_directory,
    instance::{self, TRAY_INSTANCE, UI_INSTANCE},
    logging,
    scheduler::{self, Message, Mode},
    vault::{SERVICE, Vault},
};
use chrono::{Duration, Utc};
use chrono_tz::Tz;
use std::sync::{Arc, Mutex, mpsc};
use summary::{Summary, summarize};
use tray_icon::{
    Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::{
        DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, PostThreadMessageW,
        TranslateMessage, WM_APP,
    },
};

const UI_EXECUTABLE: &str = "calendar-desktop.exe";
/// How far ahead the "Next" item looks into the cache.
const LOOKAHEAD: Duration = Duration::days(14);

type SharedSummary = Arc<Mutex<Summary>>;

fn main() {
    let Ok(_instance) = instance::acquire(TRAY_INSTANCE) else {
        return;
    };
    logging::init("tray");

    // Posting a thread message needs the queue to exist; creating it first avoids a lost first update.
    let mut message = MSG::default();
    // SAFETY: plain Win32 calls on this thread.
    let main_thread = unsafe {
        let _ = PeekMessageW(&mut message, None, WM_APP, WM_APP, PM_NOREMOVE);
        GetCurrentThreadId()
    };

    let summary = SharedSummary::default();
    let (sender, messages) = mpsc::channel();
    let worker_summary = summary.clone();
    std::thread::Builder::new()
        .name("calendar-tray-sync".into())
        .spawn(move || run_worker(&messages, &worker_summary, main_thread))
        .expect("spawn tray sync thread");

    let open = MenuItem::with_id("open", "Open Calendar", true, None);
    let next = MenuItem::with_id("next", "No upcoming events", false, None);
    let sync = MenuItem::with_id("sync", "Sync now", true, None);
    let last_synced = MenuItem::with_id("last_synced", "Not synced yet", false, None);
    let settings = MenuItem::with_id("settings", "Settings", true, None);
    let quit = MenuItem::with_id("quit", "Quit", true, None);
    let menu = Menu::new();
    let built = menu.append_items(&[
        &open,
        &PredefinedMenuItem::separator(),
        &next,
        &PredefinedMenuItem::separator(),
        &sync,
        &last_synced,
        &settings,
        &quit,
    ]);
    let tray = built.map_err(|error| error.to_string()).and_then(|()| {
        TrayIconBuilder::new()
            .with_tooltip("Calendar")
            .with_icon(icon().map_err(|error| error.to_string())?)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .map_err(|error| error.to_string())
    });
    let tray = match tray {
        Ok(tray) => tray,
        Err(error) => {
            tracing::error!(action = "tray_unavailable", %error);
            return;
        }
    };

    let apply_summary = || {
        let summary = summary.lock().unwrap().clone();
        next.set_text(&summary.next_item);
        last_synced.set_text(&summary.last_synced_item);
        let _ = tray.set_tooltip(Some(&summary.tooltip));
    };

    // SAFETY: standard message loop; `message` is only read after `GetMessageW` filled it.
    while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
        if message.message == WM_APP {
            apply_summary();
        } else {
            // SAFETY: the message came from `GetMessageW`.
            unsafe {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                launch_ui(Some("--agenda"));
            }
        }
        while let Ok(event) = MenuEvent::receiver().try_recv() {
            match event.id.0.as_str() {
                "open" => launch_ui(None),
                "settings" => launch_ui(Some("--settings")),
                "sync" => {
                    let _ = sender.send(Message::SyncNow);
                }
                "quit" => {
                    tracing::info!(action = "quit_requested");
                    let _ = sender.send(Message::Stop);
                    return;
                }
                _ => (),
            }
        }
    }
}

/// Starts the UI process next to this executable. A UI that already runs focuses itself instead.
fn launch_ui(argument: Option<&str>) {
    let Ok(path) = std::env::current_exe() else {
        return;
    };
    let mut command = std::process::Command::new(path.with_file_name(UI_EXECUTABLE));
    command.args(argument);
    if let Err(error) = command.spawn() {
        tracing::warn!(action = "ui_launch_failed", %error);
    }
}

/// The UI process syncs while it is open; polling too would double the requests.
fn should_poll(ui_running: bool, configured: bool) -> bool {
    configured && !ui_running
}

fn run_worker(messages: &mpsc::Receiver<Message>, summary: &SharedSummary, main_thread: u32) {
    let Some(directory) = data_directory() else {
        return;
    };
    let mut store = match Store::open(&directory) {
        Ok(store) => store,
        Err(error) => return tracing::error!(action = "tray_cache_unavailable", %error),
    };
    let vault = Vault::new(SERVICE);
    let mut syncer = Config::from_environment().map(|config| Syncer::new(Google::new(config)));
    let mut last_synced = None;

    scheduler::run(Mode::TrayIdle, messages, || {
        if should_poll(instance::is_running(UI_INSTANCE), syncer.is_some())
            && let Some(syncer) = syncer.as_mut()
        {
            let token = |account_id: &str| {
                vault
                    .load(account_id)
                    .ok()
                    .flatten()
                    .map(|secret| secret.refresh_token)
            };
            match syncer.sync_all(&mut store, Utc::now(), token) {
                Ok(_) => last_synced = Some(Utc::now()),
                Err(error) => tracing::warn!(action = "tray_sync_failed", %error),
            }
        }

        // Also runs while the UI is open, because the UI's sync changed the cache.
        let now = Utc::now();
        match store.events_between(now, now + LOOKAHEAD) {
            Ok(events) => {
                // The zone recorded by whichever process synced last, so the tray and the window agree.
                let zone = store.zone().ok().flatten().unwrap_or(Tz::UTC);
                *summary.lock().unwrap() = summarize(&events, now, last_synced, &zone);
                // SAFETY: posts a plain thread message; a closed queue only makes it fail.
                let _ = unsafe { PostThreadMessageW(main_thread, WM_APP, WPARAM(0), LPARAM(0)) };
            }
            Err(error) => tracing::warn!(action = "tray_cache_read_failed", %error),
        }
    });
}

fn icon() -> Result<Icon, tray_icon::BadIcon> {
    let mut pixels = Vec::with_capacity(32 * 32 * 4);
    for vertical in 0..32 {
        for horizontal in 0..32 {
            let distance = (horizontal as f32 - 15.5).powi(2) + (vertical as f32 - 15.5).powi(2);
            let inside = (30.0..180.0).contains(&distance);
            pixels.extend_from_slice(&[183, 180, 239, if inside { 255 } else { 0 }]);
        }
    }

    Icon::from_rgba(pixels, 32, 32)
}

#[cfg(test)]
mod tests {
    use super::should_poll;

    #[test]
    fn polling_waits_for_the_ui_to_close_and_for_a_configured_client() {
        assert!(should_poll(false, true));
        assert!(!should_poll(true, true));
        assert!(!should_poll(false, false));
    }
}
