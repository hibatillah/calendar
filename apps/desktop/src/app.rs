mod agenda;
mod controls;
mod editor;
mod gestures;
mod navigation;
mod settings;
mod shell;
mod sidebar;
mod timeline;

use crate::service::{self, Request, Update};
use calendar_core::{CalendarEvent, EventCommand, resolve_local};
use calendar_layout::{range_layout, snap_minutes};
use chrono::{Datelike, Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use controls::{button, calendar_toggle, event_surface, field, icon_button};
use gpui_kit::{
    component::{
        TitleBar,
        input::{Input, InputState},
    },
    prelude::*,
    *,
};

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct Preferences {
    sunday_first: bool,
    tray_all_day: bool,
}

impl Preferences {
    fn load() -> Self {
        std::fs::read_to_string(".local/preferences.json")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or(Self {
                sunday_first: false,
                tray_all_day: true,
            })
    }

    fn save(&self) {
        let _ = std::fs::write(".local/preferences.json", serde_json::to_vec(self).unwrap());
    }
}

const SIDEBAR: f32 = 240.0;
const GUTTER: f32 = 68.0;
const HEADER: f32 = 132.0;
const HOUR: f32 = 48.0;
const INK: u32 = 0xe9e9e9;
const MUTED: u32 = 0x969696;
const ACCENT: u32 = 0xbdbdbd;

#[derive(Clone, Copy)]
enum GestureKind {
    Move,
    Start,
    End,
    Create,
}

struct Gesture {
    kind: GestureKind,
    event: CalendarEvent,
    origin: Point<Pixels>,
    changed: bool,
}

pub struct CalendarApp {
    preferences: Preferences,
    settings_open: bool,
    settings_section: String,
    profile_open: bool,
    active_account: usize,
    events: Vec<CalendarEvent>,
    date: NaiveDate,
    horizontal_fraction: f32,
    sidebar_open: bool,
    month_view: bool,
    view_menu_open: bool,
    timezone: Tz,
    connection: service::Connection,
    online: bool,
    google_connection: service::GoogleConnection,
    message: String,
    selected: Option<CalendarEvent>,
    title: Entity<InputState>,
    date_input: Entity<InputState>,
    start_input: Entity<InputState>,
    end_input: Entity<InputState>,
    end_date_input: Entity<InputState>,
    location_input: Entity<InputState>,
    description_input: Entity<InputState>,
    scroll: ScrollHandle,
    activity_scroll: ScrollHandle,
    gesture: Option<Gesture>,
    focus: FocusHandle,
    hidden_calendars: std::collections::HashSet<String>,
    syncing: bool,
    sync_error: Option<String>,
    requested_month: Option<NaiveDate>,
    pending: bool,
    pending_change: Option<(String, Option<CalendarEvent>)>,
    pub agenda_only: bool,
    activity_open: bool,
    activity_errors_only: bool,
    backend_logs: Vec<calendar_telemetry::LogEntry>,
}

impl CalendarApp {
    pub fn open_event(
        &mut self,
        event: CalendarEvent,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
        self.date = event.start.with_timezone(&self.timezone).date_naive();
        self.horizontal_fraction = 0.0;
        self.edit(event, window, context);
    }

    pub fn new(window: &mut Window, context: &mut Context<Self>, agenda_only: bool) -> Self {
        let timezone = chrono_tz::Asia::Jakarta;
        let preferences = Preferences::load();
        let today = Utc::now().with_timezone(&timezone).date_naive();
        let date = week_start(today, preferences.sunday_first);
        let title =
            context.new(|context| InputState::new(window, context).placeholder("Event title"));
        let date_input =
            context.new(|context| InputState::new(window, context).placeholder("YYYY-MM-DD"));
        let start_input =
            context.new(|context| InputState::new(window, context).placeholder("09:00"));
        let end_input =
            context.new(|context| InputState::new(window, context).placeholder("10:00"));
        let end_date_input =
            context.new(|context| InputState::new(window, context).placeholder("YYYY-MM-DD"));
        let location_input = context.new(|context| {
            InputState::new(window, context).placeholder("Location or meeting link")
        });
        let description_input =
            context.new(|context| InputState::new(window, context).placeholder("Description"));
        let focus = context.focus_handle();
        focus.focus(window, context);
        let scroll = ScrollHandle::new();
        scroll.set_offset(point(px(0.0), px(-7.0 * HOUR)));
        context
            .spawn(async |entity, context| {
                loop {
                    context
                        .background_executor()
                        .timer(std::time::Duration::from_millis(120))
                        .await;
                    if entity
                        .update(context, |app, context| {
                            app.poll(context);
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        if agenda_only {
            context
                .observe_window_activation(window, |_, window, _| {
                    if !window.is_window_active() {
                        window.remove_window();
                    }
                })
                .detach();
        }

        #[cfg(not(test))]
        let connection = service::start();
        #[cfg(test)]
        let connection = service::disconnected();
        calendar_telemetry::track(
            "screen_viewed",
            if agenda_only {
                "tray_agenda"
            } else {
                "week_calendar"
            },
        );
        Self {
            preferences,
            settings_open: false,
            settings_section: "general".into(),
            profile_open: false,
            active_account: 0,
            activity_open: false,
            activity_errors_only: false,
            backend_logs: Vec::new(),
            events: Vec::new(),
            date,
            horizontal_fraction: 0.0,
            sidebar_open: true,
            month_view: false,
            view_menu_open: false,
            timezone,
            connection,
            online: false,
            google_connection: service::GoogleConnection::default(),
            message: "Connecting to the local backend…".into(),
            selected: None,
            title,
            date_input,
            start_input,
            end_input,
            end_date_input,
            location_input,
            description_input,
            scroll,
            activity_scroll: ScrollHandle::new(),
            gesture: None,
            focus,
            hidden_calendars: Default::default(),
            syncing: false,
            sync_error: None,
            requested_month: None,
            pending: false,
            pending_change: None,
            agenda_only,
        }
    }

    fn poll(&mut self, context: &mut Context<Self>) {
        let month = self.date.with_day(1).unwrap();
        if self.requested_month != Some(month) {
            let _ = self.connection.sender.send(Request::ViewMonth(month));
            self.requested_month = Some(month);
        }
        let mut changed = false;
        while let Ok(update) = self.connection.receiver.try_recv() {
            match update {
                Update::GoogleConnection(connection) => {
                    if connection.account_key != self.google_connection.account_key {
                        self.hidden_calendars.clear();
                    }
                    self.google_connection = connection;
                }
                Update::SyncStatus(syncing, error) => {
                    self.syncing = syncing;
                    self.sync_error = error;
                }
                Update::GoogleError(error) => {
                    self.google_connection.message = error;
                    self.google_connection.connecting = false;
                }
                Update::AuthorizationUrl(url) => {
                    if url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?") {
                        context.open_url(&url);
                        self.google_connection.connecting = true;
                        self.google_connection.message =
                            "Finish Google sign-in in your browser.".into();
                    }
                }
                Update::Diagnostics(records) => self.backend_logs = records,
                Update::Snapshot(events, online) => {
                    if self.gesture.is_none() && !self.pending {
                        self.events = events;
                    }
                    if self.online != online {
                        tracing::info!(action = "connection_changed", online);
                    }
                    self.online = online;
                    if self.message.starts_with("Connecting") {
                        self.message = "Your week, with a little more room.".into();
                    }
                }
                Update::Saved(event) => {
                    if let Some((identifier, _)) = &self.pending_change {
                        self.events
                            .retain(|existing| &existing.event_id != identifier);
                    }
                    self.events
                        .retain(|existing| existing.event_id != event.event_id);
                    self.events.push(event);
                    self.pending = false;
                    self.pending_change = None;
                    self.message = "Saved to Google Calendar.".into();
                    self.selected = None;
                    calendar_telemetry::track("event_save_completed", "calendar");
                }
                Update::Failed(error) => {
                    self.rollback_pending_change();
                    self.message = format!("Save failed: {error}");
                    calendar_telemetry::report_error("calendar_worker_failed");
                }
                Update::Status(online) => {
                    if self.online != online {
                        tracing::info!(action = "connection_changed", online);
                    }
                    self.online = online;
                }
            }
            changed = true;
        }
        if changed || self.activity_open {
            context.notify();
        }
    }

    fn calendar_visible(&self, event: &CalendarEvent) -> bool {
        !self.hidden_calendars.contains(&event.calendar_id)
    }
}

#[cfg(all(test, feature = "ui-tests"))]
mod tests;

fn week_start(date: NaiveDate, sunday_first: bool) -> NaiveDate {
    let offset = if sunday_first {
        date.weekday().num_days_from_sunday()
    } else {
        date.weekday().num_days_from_monday()
    };
    date - Duration::days(i64::from(offset))
}
