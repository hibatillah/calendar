mod accounts;
mod agenda;
mod controls;
mod details;
mod editor;
mod gestures;
mod navigation;
mod notices;
mod preferences;
mod settings;
mod shell;
mod sidebar;
mod timeline;
mod working_hours;
mod writes;

use crate::service::{Handle, Model, SignInState, Snapshot, snapshot::CalendarView};
use calendar_core::CalendarEvent;
use calendar_layout::range_layout;
use chrono::{Datelike, Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
use controls::{button, calendar_toggle, event_surface, field, icon_button, note, popover};
use editor::Editor;
use gestures::{Gesture, GestureKind};
use gpui_kit::{component::TitleBar, prelude::*, *};
pub use preferences::Preferences;
use std::sync::Arc;
use writes::InFlight;

const SIDEBAR: f32 = 240.0;
const GUTTER: f32 = 68.0;
const HOUR: f32 = 48.0;
const INK: u32 = 0xe9e9e9;
const MUTED: u32 = 0x969696;
const PANEL: u32 = 0x262626;
const FIELD: u32 = 0x303030;
const ACCENT: u32 = 0x51a2ff;
const ON_ACCENT: u32 = 0x0a0a0a;
const DANGER: u32 = 0xf87171;
const BORDER: u32 = 0x393939;
const NAVIGATION: u32 = 0x202020;
const DIALOG: u32 = 0x242424;
/// Background of hours outside the working hours: slightly darker than the main surface.
const OFF_HOURS: u32 = 0x131313;
const RECURRING_NOTE: &str = "Recurring event: changes apply to this occurrence only.";

pub struct CalendarApp {
    preferences: Preferences,
    settings_open: bool,
    settings_section: String,
    working_form: Option<working_hours::WorkingForm>,
    profile_open: bool,
    active_account: usize,
    snapshot: Arc<Snapshot>,
    syncing: bool,
    sign_in: SignInState,
    store_error: Option<String>,
    handle: Handle,
    date: NaiveDate,
    horizontal_fraction: f32,
    sidebar_open: bool,
    month_view: bool,
    view_menu_open: bool,
    timezone: Tz,
    selected: Option<CalendarEvent>,
    editor: Option<Editor>,
    gesture: Option<Gesture>,
    /// Writes sent and not finished; see `writes.rs`.
    writes: Vec<InFlight>,
    /// Why the last write without an open form did not finish.
    write_notice: Option<String>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    hidden_calendars: std::collections::HashSet<String>,
    pub agenda_only: bool,
}

impl CalendarApp {
    pub fn open_settings(&mut self, context: &mut Context<Self>) {
        self.settings_open = true;
        context.notify();
    }

    pub fn open_event(&mut self, event: CalendarEvent, context: &mut Context<Self>) {
        self.date = event.start.with_timezone(&self.timezone).date_naive();
        self.horizontal_fraction = 0.0;
        self.show_details(event, context);
    }

    pub fn new(
        window: &mut Window,
        context: &mut Context<Self>,
        agenda_only: bool,
        model: Entity<Model>,
        handle: Handle,
        preferences: Preferences,
    ) -> Self {
        // Until the first snapshot arrives with the cache's zone.
        let timezone = calendar_sync::system_zone().unwrap_or(Tz::UTC);
        let today = Utc::now().with_timezone(&timezone).date_naive();
        let date = week_start(today, preferences.sunday_first);
        let focus = context.focus_handle();
        focus.focus(window, context);
        let scroll = ScrollHandle::new();
        scroll.set_offset(point(px(0.0), px(-7.0 * HOUR)));
        context
            .observe(&model, |app, model, context| {
                let finished = model.update(context, |model, _| {
                    std::mem::take(&mut model.finished_writes)
                });
                app.copy_model(model.read(context));
                for result in finished {
                    app.finish_write(result, context);
                }
                app.settle_writes();
                context.notify();
            })
            .detach();
        context
            .observe_window_activation(window, |app, window, _| {
                if app.agenda_only && !window.is_window_active() {
                    window.remove_window();
                } else if !app.agenda_only && window.is_window_active() {
                    app.handle.sync_now();
                }
            })
            .detach();

        let mut app = Self {
            preferences,
            settings_open: false,
            settings_section: "general".into(),
            working_form: None,
            profile_open: false,
            active_account: 0,
            snapshot: Arc::default(),
            syncing: false,
            sign_in: SignInState::Idle,
            store_error: None,
            handle,
            date,
            horizontal_fraction: 0.0,
            sidebar_open: true,
            month_view: false,
            view_menu_open: false,
            timezone,
            selected: None,
            editor: None,
            gesture: None,
            writes: Vec::new(),
            write_notice: None,
            scroll,
            focus,
            hidden_calendars: Default::default(),
            agenda_only,
        };
        app.copy_model(model.read(context));

        app
    }

    fn copy_model(&mut self, model: &Model) {
        self.snapshot = model.snapshot.clone();
        if let Some(zone) = self.snapshot.zone {
            self.timezone = zone;
        }
        self.syncing = model.syncing;
        self.sign_in = model.sign_in.clone();
        self.store_error = model.error.clone();
        self.active_account = self
            .active_account
            .min(self.snapshot.accounts.len().saturating_sub(1));
    }

    fn calendar_visible(&self, event: &CalendarEvent) -> bool {
        !self.hidden_calendars.contains(&event.calendar_id)
    }
}

#[cfg(all(test, feature = "ui-tests"))]
mod tests;
#[cfg(all(test, feature = "ui-tests"))]
mod working_hours_tests;
#[cfg(all(test, feature = "ui-tests"))]
mod write_tests;

fn week_start(date: NaiveDate, sunday_first: bool) -> NaiveDate {
    let offset = if sunday_first {
        date.weekday().num_days_from_sunday()
    } else {
        date.weekday().num_days_from_monday()
    };
    date - Duration::days(i64::from(offset))
}
