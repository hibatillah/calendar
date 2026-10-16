use crate::service::{self, Request, Update};
use calendar_core::{CalendarEvent, EventCommand, resolve_local};
use calendar_layout::{range_layout, snap_minutes};
use chrono::{Datelike, Duration, NaiveDate, Timelike, Utc};
use chrono_tz::Tz;
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

    fn go_today(&mut self) {
        let today = Utc::now().with_timezone(&self.timezone).date_naive();
        self.date = if self.month_view {
            today
        } else {
            week_start(today, self.preferences.sunday_first)
        };
        self.horizontal_fraction = 0.0;
    }
    fn display_month_cells(&self) -> Vec<NaiveDate> {
        let first = self.date.with_day(1).unwrap();
        let start = week_start(first, self.preferences.sunday_first);
        (0..42)
            .map(|offset| start + Duration::days(offset))
            .collect()
    }
    fn calendar_visible(&self, event: &CalendarEvent) -> bool {
        !self.hidden_calendars.contains(&event.calendar_id)
    }

    fn new_event(&self, date: NaiveDate, minute: u32) -> CalendarEvent {
        let start = resolve_local(date, minute, self.timezone).unwrap_or_else(|_| Utc::now());
        CalendarEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            calendar_id: self
                .google_connection
                .accounts
                .iter()
                .filter(|account| account.write_access)
                .flat_map(|account| account.calendars.iter())
                .find(|calendar| {
                    calendar.primary && matches!(calendar.access_role.as_str(), "owner" | "writer")
                })
                .map(|calendar| calendar.id.clone())
                .unwrap_or_else(|| "unconnected".into()),
            title: "New event".into(),
            start,
            end: start + Duration::hours(1),
            timezone: self.timezone.name().into(),
            all_day: false,
            description: String::new(),
            location: String::new(),
            color: 0x8c9ef8,
            revision: -1,
            read_only: false,
        }
    }

    fn edit(&mut self, event: CalendarEvent, window: &mut Window, context: &mut Context<Self>) {
        if event.revision < 0
            && !self
                .google_connection
                .accounts
                .iter()
                .any(|account| account.write_access)
        {
            self.settings_open = true;
            self.settings_section = "accounts".into();
            self.message =
                "Connect a Google account and approve calendar editing to create events.".into();
            context.notify();
            return;
        }
        self.message = if event.read_only {
            "This calendar is read-only.".into()
        } else {
            String::new()
        };
        calendar_telemetry::track("screen_viewed", "event_editor");
        let start = event.start.with_timezone(&self.timezone);
        let end = event.end.with_timezone(&self.timezone);
        for (input, value) in [
            (&self.title, event.title.clone()),
            (&self.date_input, start.format("%Y-%m-%d").to_string()),
            (&self.start_input, start.format("%H:%M").to_string()),
            (&self.end_input, end.format("%H:%M").to_string()),
            (&self.end_date_input, end.format("%Y-%m-%d").to_string()),
            (&self.location_input, event.location.clone()),
            (&self.description_input, event.description.clone()),
        ] {
            input.update(context, |input, context| {
                input.set_value(value, window, context)
            });
        }
        self.selected = Some(event);
        context.notify();
    }

    fn rollback_pending_change(&mut self) {
        if let Some((event_id, original)) = self.pending_change.take() {
            self.events.retain(|event| event.event_id != event_id);
            if let Some(original) = original {
                self.events.push(original);
            }
        }
        self.pending = false;
    }

    fn commit(&mut self, event: CalendarEvent, context: &mut Context<Self>) {
        if !self.online {
            self.message = "Offline viewing. Start the backend to edit.".into();
            context.notify();
            return;
        }
        if self.pending {
            return;
        }
        if let Err(error) = event.validate() {
            self.message = error;
            context.notify();
            return;
        }
        if event.read_only {
            self.message = "This calendar is read-only.".into();
            context.notify();
            return;
        }
        let original = self
            .events
            .iter()
            .find(|existing| existing.event_id == event.event_id)
            .cloned();
        self.pending_change = Some((event.event_id.clone(), original));
        self.events
            .retain(|existing| existing.event_id != event.event_id);
        self.events.push(event.clone());
        let command = EventCommand {
            operation_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: event.revision,
            event,
        };
        tracing::info!(action="save_requested", operation_id=%command.operation_id, event_id=%command.event.event_id);
        calendar_telemetry::track("event_save_requested", "event_editor_or_gesture");
        self.pending = true;
        self.message = "Saving…".into();
        if self
            .connection
            .sender
            .send(Request::Save(Box::new(command)))
            .is_err()
        {
            self.rollback_pending_change();
            self.message = "Save failed: calendar worker stopped. Restart the app.".into();
        }
        context.notify();
    }

    fn save_form(&mut self, context: &mut Context<Self>) {
        let Some(mut event) = self.selected.clone() else {
            return;
        };
        let result = (|| -> Result<(), String> {
            let date: NaiveDate = self
                .date_input
                .read(context)
                .value()
                .parse()
                .map_err(|_| "Use YYYY-MM-DD for the date")?;
            let end_date: NaiveDate = self
                .end_date_input
                .read(context)
                .value()
                .parse()
                .map_err(|_| "Use YYYY-MM-DD for the end date")?;
            let parse_minutes = |value: &str| -> Result<u32, String> {
                let time = chrono::NaiveTime::parse_from_str(value, "%H:%M")
                    .map_err(|_| "Use HH:MM for time")?;
                Ok(time.hour() * 60 + time.minute())
            };
            let start_minutes = parse_minutes(self.start_input.read(context).value().as_str())?;
            let end_minutes = parse_minutes(self.end_input.read(context).value().as_str())?;
            event.start = resolve_local(
                date,
                if event.all_day { 0 } else { start_minutes },
                self.timezone,
            )?;
            event.end = if event.all_day {
                resolve_local(end_date.max(date + Duration::days(1)), 0, self.timezone)?
            } else {
                resolve_local(end_date, end_minutes, self.timezone)?
            };
            event.title = self.title.read(context).value().to_string();
            event.location = self.location_input.read(context).value().to_string();
            event.description = self.description_input.read(context).value().to_string();
            event.validate()
        })();
        match result {
            Ok(()) => self.commit(event, context),
            Err(error) => {
                self.message = error;
                context.notify();
            }
        }
    }

    fn sidebar_width(&self) -> f32 {
        if self.sidebar_open { SIDEBAR } else { 0.0 }
    }

    fn set_month_view(&mut self, month_view: bool, context: &mut Context<Self>) {
        self.month_view = month_view;
        self.view_menu_open = false;
        self.horizontal_fraction = 0.0;
        tracing::info!(
            action = "view_changed",
            mode = if month_view { "month" } else { "week" }
        );
        context.notify();
    }

    fn toggle_sidebar(&mut self, context: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        tracing::info!(action = "sidebar_toggled", visible = self.sidebar_open);
        context.notify();
    }

    fn scroll_dates(&mut self, delta: f32, day_width: f32, context: &mut Context<Self>) {
        if self.gesture.is_some() || !delta.is_finite() {
            return;
        }
        let position = self.horizontal_fraction - delta / day_width;
        let days = position.floor() as i64;
        self.date += Duration::days(days);
        self.horizontal_fraction = position - days as f32;
        if days != 0 {
            tracing::info!(action = "timeline_scrolled", offset_days = days);
        }
        context.notify();
    }

    fn scroll_timeline(
        &mut self,
        event: &ScrollWheelEvent,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
        let delta = event.delta.pixel_delta(px(32.0));
        let horizontal = if event.modifiers.shift && delta.x == px(0.0) {
            delta.y
        } else {
            delta.x
        };
        if event.modifiers.shift || horizontal.abs() > delta.y.abs() {
            context.stop_propagation();
            let day_width =
                ((f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER) / 7.0)
                    .max(80.0);
            self.scroll_dates(f32::from(horizontal), day_width, context);
        }
    }

    fn date_visible(&self, date: NaiveDate) -> bool {
        if self.month_view {
            return date.year() == self.date.year() && date.month() == self.date.month();
        }
        // Follow the seven day-column centers, not a clipped sliver at either edge.
        let first_highlighted_date =
            self.date + Duration::days(self.horizontal_fraction.round() as i64);
        (0..7).contains(&(date - first_highlighted_date).num_days())
    }

    fn navigate(&mut self, days: i64, context: &mut Context<Self>) {
        calendar_telemetry::track("control_clicked", "week_navigation");
        self.date = if self.month_view {
            let first = self.date.with_day(1).unwrap();
            if days < 0 {
                (first - Duration::days(1)).with_day(1).unwrap()
            } else {
                (first + Duration::days(32)).with_day(1).unwrap()
            }
        } else {
            self.date + Duration::days(days)
        };
        self.horizontal_fraction = 0.0;
        tracing::info!(action = "week_navigated", offset_days = days);
        context.notify();
    }

    fn sidebar(&self, context: &mut Context<Self>) -> Div {
        let mut cells = self.display_month_cells();
        // Keep the trailing visible date in the mini calendar even in six-row months.
        if self.date_visible(*cells.last().unwrap() + Duration::days(1)) {
            for date in &mut cells {
                *date += Duration::days(7);
            }
        }
        div()
            .w(px(SIDEBAR))
            .h_full()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .bg(rgb(0x222222))
            .border_r_1()
            .border_color(rgb(0x292929))
            .child(
                div()
                    .h(px(54.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .when(cfg!(target_os = "macos"), |element| element.pt_5())
                    .child(
                        icon_button("sidebar-today", "icons/chevron-down.svg")
                            .text_color(rgb(MUTED))
                            .on_click(context.listener(|app, _, _, context| {
                                app.go_today();
                                app.horizontal_fraction = 0.0;
                                context.notify();
                            })),
                    )
                    .child(
                        icon_button("sidebar-close", "icons/panel-left.svg").on_click(
                            context.listener(|app, _, _, context| app.toggle_sidebar(context)),
                        ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                window.start_window_move()
                            }),
                    )
                    .child(
                        icon_button("new-event", "icons/square-pen.svg")
                            .text_color(rgb(MUTED))
                            .on_click(context.listener(|app, _, window, context| {
                                let event = app.new_event(app.date, 540);
                                app.edit(event, window, context);
                            })),
                    ),
            )
            .child(
                div()
                    .px_2()
                    .flex()
                    .justify_end()
                    .h(px(28.0))
                    .gap_1()
                    .child(
                        icon_button("month-prev", "icons/chevron-up.svg")
                            .text_color(rgb(MUTED))
                            .on_click(context.listener(|app, _, _, context| {
                                app.date = app.date.with_day(1).unwrap() - Duration::days(1);
                                app.horizontal_fraction = 0.0;
                                context.notify();
                            })),
                    )
                    .child(
                        icon_button("month-next", "icons/chevron-down.svg")
                            .text_color(rgb(MUTED))
                            .on_click(context.listener(|app, _, _, context| {
                                app.date = (app.date.with_day(1).unwrap() + Duration::days(32))
                                    .with_day(1)
                                    .unwrap();
                                app.horizontal_fraction = 0.0;
                                context.notify();
                            })),
                    ),
            )
            .child(
                div().px_2().flex().mb_1().children(
                    (if self.preferences.sunday_first {
                        ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
                    } else {
                        ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
                    })
                    .into_iter()
                    .map(|day| {
                        div()
                            .w(px(32.0))
                            .text_center()
                            .text_size(px(10.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(0x858585))
                            .child(day)
                    }),
                ),
            )
            .children(cells.chunks(7).enumerate().map(|(row, dates)| {
                div()
                    .mx_2()
                    .flex()
                    .h(px(28.0))
                    .rounded_lg()
                    .children(dates.iter().enumerate().map(|(column, date)| {
                        let date = *date;
                        div()
                            .id(("month-day", row * 7 + column))
                            .w(px(32.0))
                            .h(px(28.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(self.date_visible(date), |element| element.bg(rgb(0x303030)))
                            .when(
                                !self.date_visible(date - Duration::days(1)) || column == 0,
                                |element| element.rounded_l_md(),
                            )
                            .when(
                                !self.date_visible(date + Duration::days(1)) || column == 6,
                                |element| element.rounded_r_md(),
                            )
                            .text_color(rgb(if date.month() == self.date.month() {
                                0xcccccc
                            } else {
                                0x626262
                            }))
                            .when(
                                date == Utc::now().with_timezone(&self.timezone).date_naive(),
                                |element| element.bg(rgb(0xf45151)).text_color(rgb(0xffffff)),
                            )
                            .hover(|style| style.bg(rgb(0xffffff).opacity(0.08)))
                            .on_click(context.listener(move |app, _, _, context| {
                                app.date = date;
                                app.horizontal_fraction = 0.0;
                                tracing::info!(action = "date_selected");
                                context.notify();
                            }))
                            .child(date.day().to_string())
                    }))
            }))
            .child(
                div()
                    .id("account-calendars")
                    .mt_4()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .children(
                        self.google_connection
                            .accounts
                            .iter()
                            .filter(|account| !account.calendars.is_empty())
                            .map(|account| {
                                div()
                                    .mb_3()
                                    .child(
                                        div()
                                            .mx_3()
                                            .mb_2()
                                            .text_size(px(11.0))
                                            .text_color(rgb(MUTED))
                                            .truncate()
                                            .child(account.email.clone()),
                                    )
                                    .child(div().px_2().children(account.calendars.iter().map(
                                        |calendar| {
                                            let calendar_id = calendar.id.clone();
                                            calendar_toggle(
                                                SharedString::from(format!(
                                                    "calendar-{}",
                                                    calendar.id
                                                )),
                                                SharedString::from(calendar.summary.clone()),
                                                calendar.color(),
                                                !self.hidden_calendars.contains(&calendar.id),
                                            )
                                            .on_click(
                                                context.listener(move |app, _, _, context| {
                                                    if !app.hidden_calendars.remove(&calendar_id) {
                                                        app.hidden_calendars
                                                            .insert(calendar_id.clone());
                                                    }
                                                    tracing::info!(
                                                        action = "calendar_visibility_changed"
                                                    );
                                                    context.notify();
                                                }),
                                            )
                                        },
                                    )))
                            }),
                    ),
            )
            .child(
                button("activity-open", "Activity")
                    .mx_2()
                    .text_color(rgb(MUTED))
                    .on_click(context.listener(|app, _, _, context| {
                        app.activity_open = !app.activity_open;
                        calendar_telemetry::track("screen_viewed", "activity");
                        context.notify();
                    })),
            )
    }

    fn activity_panel(&self, context: &mut Context<Self>) -> impl IntoElement {
        let mut records = calendar_telemetry::recent_logs();
        records.extend(self.backend_logs.iter().cloned());
        records.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
        records.retain(|record| {
            !self.activity_errors_only || matches!(record.level.as_str(), "WARN" | "ERROR")
        });
        div()
            .absolute()
            .top(px(52.0))
            .bottom_3()
            .right_3()
            .w(px(400.0))
            .occlude()
            .on_scroll_wheel(|_, _, context| context.stop_propagation())
            .flex()
            .flex_col()
            .bg(rgb(0x242424))
            .border_1()
            .border_color(rgb(0x3b3b3b))
            .rounded_lg()
            .shadow_lg()
            .on_mouse_down(MouseButton::Left, |_, _, context| {
                context.stop_propagation()
            })
            .child(
                div()
                    .p_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Activity & diagnostics"),
                    )
                    .child(button("activity-close", "Close").on_click(context.listener(
                        |app, _, _, context| {
                            app.activity_open = false;
                            context.notify();
                        },
                    ))),
            )
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .text_size(px(11.0))
                    .text_color(rgb(MUTED))
                    .child(calendar_telemetry::status()),
            )
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "activity-filter",
                            if self.activity_errors_only {
                                "Warnings & errors"
                            } else {
                                "All activity"
                            },
                        )
                        .on_click(context.listener(
                            |app, _, _, context| {
                                app.activity_errors_only = !app.activity_errors_only;
                                context.notify();
                            },
                        )),
                    )
                    .child(
                        button("activity-copy", "Copy diagnostics").on_click(context.listener(
                            |app, _, _, context| {
                                let mut records = calendar_telemetry::recent_logs();
                                records.extend(app.backend_logs.iter().cloned());
                                if let Ok(text) = serde_json::to_string_pretty(&records) {
                                    context.write_to_clipboard(ClipboardItem::new_string(text));
                                }
                                calendar_telemetry::track("control_clicked", "copy_diagnostics");
                            },
                        )),
                    ),
            )
            .child(
                div()
                    .px_3()
                    .pb_2()
                    .text_size(px(10.0))
                    .text_color(rgb(MUTED))
                    .child(if self.online {
                        "Live desktop · backend refreshes every 5s · newest first"
                    } else {
                        "Live desktop · backend disconnected; last received logs shown"
                    }),
            )
            .child(
                div()
                    .id("activity-entries")
                    .debug_selector(|| "activity-entries".into())
                    .track_scroll(&self.activity_scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .px_3()
                    .when(records.is_empty(), |element| {
                        element.child(
                            div()
                                .py_4()
                                .text_color(rgb(MUTED))
                                .child("No matching activity yet."),
                        )
                    })
                    .children(records.into_iter().take(500).map(|record| {
                        let details = record
                            .details
                            .iter()
                            .map(|(key, value)| format!("{key}: {value}"))
                            .collect::<Vec<_>>()
                            .join(" · ");
                        div()
                            .py_2()
                            .border_b_1()
                            .border_color(rgb(0x333333))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(rgb(if record.level == "ERROR" {
                                        0xf08080
                                    } else {
                                        MUTED
                                    }))
                                    .child(format!(
                                        "{}   {}   {}",
                                        record.timestamp, record.component, record.level
                                    )),
                            )
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .child(record.action.replace('_', " ")),
                            )
                            .when(!details.is_empty(), |element| {
                                element.child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(rgb(MUTED))
                                        .child(details),
                                )
                            })
                    })),
            )
    }

    fn begin_gesture(
        &mut self,
        event: CalendarEvent,
        kind: GestureKind,
        position: Point<Pixels>,
        context: &mut Context<Self>,
    ) {
        if self.pending {
            return;
        }
        if (!self.online || event.read_only) && !matches!(kind, GestureKind::Move) {
            return;
        }
        self.gesture = Some(Gesture {
            kind,
            event,
            origin: position,
            changed: false,
        });
        context.notify();
    }

    fn move_gesture(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        context: &mut Context<Self>,
    ) {
        if !self.online || self.pending {
            return;
        }
        let sidebar_width = self.sidebar_width();
        let Some(gesture) = &mut self.gesture else {
            return;
        };
        if gesture.event.read_only {
            return;
        }
        let day_width = (f32::from(window.viewport_size().width) - sidebar_width - GUTTER) / 7.0;
        let horizontal = f32::from(position.x - gesture.origin.x);
        let vertical = f32::from(position.y - gesture.origin.y);
        let minute_delta = snap_minutes(vertical * 60.0 / HOUR);
        let day_delta = (horizontal / day_width).round() as i64;
        if horizontal.abs() + vertical.abs() > 4.0 {
            gesture.changed = true;
        }
        let mut preview = gesture.event.clone();
        match gesture.kind {
            GestureKind::Move => {
                let local = preview.start.with_timezone(&self.timezone).naive_local()
                    + Duration::days(day_delta)
                    + Duration::minutes(minute_delta);
                match resolve_local(
                    local.date(),
                    local.hour() * 60 + local.minute(),
                    self.timezone,
                ) {
                    Ok(start) => {
                        let duration = preview.end - preview.start;
                        preview.start = start;
                        preview.end = start + duration;
                    }
                    Err(error) => {
                        self.message = error;
                        context.notify();
                        return;
                    }
                }
            }
            GestureKind::Start => {
                preview.start = (preview.start + Duration::minutes(minute_delta))
                    .min(preview.end - Duration::minutes(15));
            }
            GestureKind::End | GestureKind::Create => {
                preview.end = (preview.end + Duration::minutes(minute_delta))
                    .max(preview.start + Duration::minutes(15));
            }
        }
        self.events
            .retain(|event| event.event_id != preview.event_id);
        self.events.push(preview);
        context.notify();
    }

    fn finish_gesture(&mut self, window: &mut Window, context: &mut Context<Self>) {
        let Some(gesture) = self.gesture.take() else {
            return;
        };
        let preview = self
            .events
            .iter()
            .find(|event| event.event_id == gesture.event.event_id)
            .cloned()
            .unwrap_or_else(|| gesture.event.clone());
        self.events
            .retain(|event| event.event_id != gesture.event.event_id);
        if gesture.event.revision >= 0 {
            self.events.push(gesture.event.clone());
        }
        if matches!(gesture.kind, GestureKind::Create) {
            self.edit(preview, window, context);
        } else if gesture.changed {
            calendar_telemetry::track(
                "event_gesture_completed",
                match gesture.kind {
                    GestureKind::Move => "move",
                    GestureKind::Start | GestureKind::End => "resize",
                    GestureKind::Create => "create",
                },
            );
            self.commit(preview, context);
        } else {
            self.edit(gesture.event, window, context);
        }
        context.notify();
    }

    fn week_grid(&self, window: &Window, context: &mut Context<Self>) -> impl IntoElement {
        let width =
            (f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER).max(560.0);
        let day_width = width / 7.0;
        let first_date = self.date;
        let visible: Vec<_> = self
            .events
            .iter()
            .chain(
                self.selected
                    .iter()
                    .filter(|event| event.revision < 0 && !self.pending),
            )
            .filter(|event| self.calendar_visible(event))
            .cloned()
            .collect();
        let placements = range_layout(&visible, first_date, 8, self.timezone);
        let mut grid = div().relative().w(px(width + GUTTER)).h(px(24.0 * HOUR));
        for hour in 0..24 {
            grid = grid.child(
                div()
                    .absolute()
                    .top(px(hour as f32 * HOUR))
                    .left_0()
                    .w_full()
                    .h(px(HOUR))
                    .border_t_1()
                    .border_color(rgb(0xffffff).opacity(0.055))
                    .child(
                        div()
                            .w(px(GUTTER - 8.0))
                            .text_right()
                            .pt_1()
                            .text_size(px(10.0))
                            .text_color(rgb(MUTED))
                            .child(format!("{hour:02}:00")),
                    ),
            );
        }
        let horizontal_offset = self.horizontal_fraction * day_width;
        let mut days_grid = div()
            .absolute()
            .left(px(GUTTER))
            .top_0()
            .w(px(width))
            .h_full()
            .overflow_hidden();
        for day in 0..8 {
            days_grid = days_grid.child(
                div()
                    .id(("day-grid", day as usize))
                    .absolute()
                    .left(px(day as f32 * day_width - horizontal_offset))
                    .top_0()
                    .w(px(day_width))
                    .h_full()
                    .border_l_1()
                    .border_color(rgb(0xffffff).opacity(0.06))
                    .cursor_crosshair()
                    .on_mouse_down(
                        MouseButton::Left,
                        context.listener(move |app, event: &MouseDownEvent, _, context| {
                            let relative = f32::from(event.position.y)
                                - HEADER
                                - f32::from(app.scroll.offset().y);
                            let minute =
                                (snap_minutes(relative * 60.0 / HOUR).clamp(0, 1425)) as u32;
                            let mut new_event =
                                app.new_event(first_date + Duration::days(day as i64), minute);
                            new_event.end = new_event.start + Duration::minutes(30);
                            app.begin_gesture(
                                new_event,
                                GestureKind::Create,
                                event.position,
                                context,
                            );
                        }),
                    ),
            );
        }
        for placement in placements {
            let Some(event) = visible
                .iter()
                .find(|event| event.event_id == placement.event_id)
                .cloned()
            else {
                continue;
            };
            let event_width = (day_width - 7.0) / placement.column_count as f32;
            let height = ((placement.end_minute - placement.start_minute) * HOUR / 60.0).max(22.0);
            let move_event = event.clone();
            let start_event = event.clone();
            let end_event = event.clone();
            days_grid = days_grid.child(
                div()
                    .id(SharedString::from(format!(
                        "{}-{}",
                        event.event_id,
                        first_date + Duration::days(placement.day as i64)
                    )))
                    .absolute()
                    .left(px(placement.day as f32 * day_width - horizontal_offset
                        + placement.column as f32 * event_width
                        + 3.0))
                    .top(px(placement.start_minute * HOUR / 60.0))
                    .w(px(event_width - 2.0
                        + if placement.column + 1 < placement.column_count {
                            (event_width * 0.18).min(22.0)
                        } else {
                            0.0
                        }))
                    .h(px(height))
                    .rounded_md()
                    .overflow_hidden()
                    .bg(rgb(event_surface(event.color)))
                    .border_l_2()
                    .border_color(rgb(event.color))
                    .cursor_pointer()
                    .hover(|style| style.bg(rgb(event_surface(event.color))))
                    .on_mouse_down(
                        MouseButton::Left,
                        context.listener(move |app, mouse: &MouseDownEvent, _, context| {
                            context.stop_propagation();
                            app.begin_gesture(
                                move_event.clone(),
                                GestureKind::Move,
                                mouse.position,
                                context,
                            );
                        }),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .w_full()
                            .h(px(6.0))
                            .cursor_ns_resize()
                            .on_mouse_down(
                                MouseButton::Left,
                                context.listener(move |app, mouse: &MouseDownEvent, _, context| {
                                    context.stop_propagation();
                                    app.begin_gesture(
                                        start_event.clone(),
                                        GestureKind::Start,
                                        mouse.position,
                                        context,
                                    );
                                }),
                            ),
                    )
                    .child(
                        div()
                            .px_1()
                            .py_1()
                            .text_size(px(11.0))
                            .text_color(rgb(0xf0e7e5))
                            .font_weight(FontWeight::MEDIUM)
                            .child(event.title.clone())
                            .when(height > 42.0, |element| {
                                element.child(
                                    div()
                                        .mt_0()
                                        .text_color(rgb(event.color))
                                        .text_size(px(10.0))
                                        .font_weight(FontWeight::NORMAL)
                                        .child(format!(
                                            "{} – {}",
                                            event
                                                .start
                                                .with_timezone(&self.timezone)
                                                .format("%H:%M"),
                                            event.end.with_timezone(&self.timezone).format("%H:%M")
                                        )),
                                )
                            }),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .w_full()
                            .h(px(7.0))
                            .cursor_ns_resize()
                            .on_mouse_down(
                                MouseButton::Left,
                                context.listener(move |app, mouse: &MouseDownEvent, _, context| {
                                    context.stop_propagation();
                                    app.begin_gesture(
                                        end_event.clone(),
                                        GestureKind::End,
                                        mouse.position,
                                        context,
                                    );
                                }),
                            ),
                    ),
            );
        }
        let now = Utc::now().with_timezone(&self.timezone);
        let today_offset = (now.date_naive() - first_date).num_days();
        if (0..8).contains(&today_offset) {
            days_grid = days_grid.child(
                div()
                    .absolute()
                    .left(px(today_offset as f32 * day_width - horizontal_offset))
                    .top(px((now.hour() * 60 + now.minute()) as f32 * HOUR / 60.0))
                    .w(px(day_width))
                    .h(px(1.0))
                    .bg(rgb(0xe8a69a)),
            );
        }
        div()
            .id("week-scroll")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            .child(
                grid.child(days_grid)
                    .on_scroll_wheel(context.listener(Self::scroll_timeline)),
            )
    }

    fn month_grid(&self, context: &mut Context<Self>) -> impl IntoElement {
        let cells = self.display_month_cells();
        let today = Utc::now().with_timezone(&self.timezone).date_naive();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div().h(px(26.0)).flex().flex_shrink_0().children(
                    (if self.preferences.sunday_first {
                        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
                    } else {
                        ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                    })
                    .into_iter()
                    .map(|day| {
                        div()
                            .flex_1()
                            .text_center()
                            .text_color(rgb(MUTED))
                            .child(day)
                    }),
                ),
            )
            .children(cells.chunks(7).enumerate().map(|(row, dates)| {
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .children(dates.iter().enumerate().map(|(column, date)| {
                        let date = *date;
                        let mut events: Vec<_> = self
                            .events
                            .iter()
                            .filter(|event| {
                                (self.calendar_visible(event))
                                    && event.start.with_timezone(&self.timezone).date_naive()
                                        <= date
                                    && (event.end - Duration::nanoseconds(1))
                                        .with_timezone(&self.timezone)
                                        .date_naive()
                                        >= date
                            })
                            .collect();
                        events.sort_by_key(|event| (!event.all_day, event.start));
                        div()
                            .id(("month-cell", row * 7 + column))
                            .flex_1()
                            .min_w_0()
                            .h_full()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .border_t_1()
                            .border_l_1()
                            .border_color(rgb(0x242424))
                            .on_click(context.listener(move |app, _, window, context| {
                                let event = app.new_event(date, 540);
                                app.edit(event, window, context);
                            }))
                            .child(
                                div().flex().justify_end().p_1().flex_shrink_0().child(
                                    div()
                                        .px_1()
                                        .h(px(22.0))
                                        .rounded_md()
                                        .text_size(px(12.0))
                                        .text_color(rgb(if date.month() == self.date.month() {
                                            INK
                                        } else {
                                            0x626262
                                        }))
                                        .when(date == today, |element| {
                                            element.bg(rgb(0xf45151)).text_color(rgb(0xffffff))
                                        })
                                        .child(if date.day() == 1 {
                                            date.format("%B %-d").to_string()
                                        } else {
                                            date.day().to_string()
                                        }),
                                ),
                            )
                            .child(
                                div()
                                    .id(("month-events", row * 7 + column))
                                    .flex_1()
                                    .min_h_0()
                                    .overflow_y_scroll()
                                    .children(events.into_iter().map(|event| {
                                        let event = event.clone();
                                        div()
                                            .id(SharedString::from(format!(
                                                "month-{}-{date}",
                                                event.event_id
                                            )))
                                            .mx_1()
                                            .mb_1()
                                            .px_1()
                                            .h(px(20.0))
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .rounded_sm()
                                            .border_l_2()
                                            .border_color(rgb(event.color))
                                            .text_size(px(10.0))
                                            .cursor_pointer()
                                            .bg(rgb(event_surface(event.color)))
                                            .text_color(rgb(INK))
                                            .child(format!(
                                                "{}{}",
                                                if event.all_day {
                                                    String::new()
                                                } else {
                                                    format!(
                                                        "{} ",
                                                        event
                                                            .start
                                                            .with_timezone(&self.timezone)
                                                            .format("%H:%M")
                                                    )
                                                },
                                                event.title
                                            ))
                                            .on_click(context.listener(
                                                move |app, _, window, context| {
                                                    context.stop_propagation();
                                                    app.edit(event.clone(), window, context);
                                                },
                                            ))
                                    })),
                            )
                    }))
            }))
    }

    fn editor(&self, context: &mut Context<Self>) -> impl IntoElement {
        let event = self.selected.as_ref().unwrap();
        div()
            .absolute()
            .id("event-popover")
            .top(px(70.0))
            .right(px(20.0))
            .bottom(px(20.0))
            .w(px(370.0))
            .occlude()
            .overflow_y_scroll()
            .on_scroll_wheel(|_, _, context| context.stop_propagation())
            .on_mouse_down(MouseButton::Left, |_, _, context| {
                context.stop_propagation()
            })
            .child(
                div()
                    .w_full()
                    .p_4()
                    .rounded_xl()
                    .bg(rgb(0x262626))
                    .border_1()
                    .border_color(rgb(0xffffff).opacity(0.1))
                    .shadow_lg()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .mb_5()
                            .child("Event details")
                            .child(button("close-editor", "×").on_click(context.listener(
                                |app, _, window, context| {
                                    app.selected = None;
                                    app.activity_open = false;
                                    app.focus.focus(window, context);
                                    context.notify();
                                },
                            ))),
                    )
                    .child(field("TITLE", Input::new(&self.title)))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .child(field("START DATE", Input::new(&self.date_input))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(field("END DATE", Input::new(&self.end_date_input))),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .child(field("START", Input::new(&self.start_input))),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .child(field("END", Input::new(&self.end_input))),
                            ),
                    )
                    .child(
                        button(
                            "all-day",
                            if event.all_day {
                                "☑  All day"
                            } else {
                                "□  All day"
                            },
                        )
                        .mb_3()
                        .on_click(context.listener(
                            |app, _, _, context| {
                                if let Some(event) = &mut app.selected {
                                    event.all_day = !event.all_day;
                                }
                                context.notify();
                            },
                        )),
                    )
                    .child(field("LOCATION", Input::new(&self.location_input)))
                    .child(field("NOTES", Input::new(&self.description_input)))
                    .when(event.revision < 0, |element| {
                        element.child(
                            button("select-calendar", "Change calendar ▾")
                                .mb_2()
                                .on_click(context.listener(|app, _, _, context| {
                                    let calendars: Vec<_> = app
                                        .google_connection
                                        .accounts
                                        .iter()
                                        .filter(|account| account.write_access)
                                        .flat_map(|account| account.calendars.iter())
                                        .filter(|calendar| {
                                            matches!(
                                                calendar.access_role.as_str(),
                                                "owner" | "writer"
                                            )
                                        })
                                        .collect();
                                    if let Some(event) = &mut app.selected
                                        && !calendars.is_empty()
                                    {
                                        let index = calendars
                                            .iter()
                                            .position(|calendar| calendar.id == event.calendar_id)
                                            .unwrap_or(0);
                                        let calendar = calendars[(index + 1) % calendars.len()];
                                        event.calendar_id = calendar.id.clone();
                                        event.color = calendar.color();
                                    }
                                    context.notify();
                                })),
                        )
                    })
                    .child(
                        div().mb_2().text_size(px(12.0)).child(
                            self.google_connection
                                .calendars
                                .iter()
                                .find(|calendar| calendar.id == event.calendar_id)
                                .map(|calendar| calendar.summary.clone())
                                .unwrap_or_else(|| "Calendar".into()),
                        ),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED))
                            .mb_4()
                            .child(self.timezone.name()),
                    )
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(ACCENT))
                            .mb_4()
                            .child(self.message.clone()),
                    )
                    .child(
                        button(
                            "save-event",
                            if event.read_only {
                                "Read-only Google event"
                            } else if self.pending {
                                "Saving…"
                            } else if self.online {
                                "Save event"
                            } else {
                                "Offline · viewing only"
                            },
                        )
                        .w_full()
                        .justify_center()
                        .bg(rgb(ACCENT))
                        .text_color(rgb(0x202020))
                        .on_click(context.listener(|app, _, _, context| app.save_form(context))),
                    ),
            )
    }

    fn agenda(&self, context: &mut Context<Self>) -> Div {
        let now = Utc::now();
        let today = now.with_timezone(&self.timezone).date_naive();
        let mut events: Vec<_> = self
            .events
            .iter()
            .filter(|event| {
                event.end > now
                    && self.calendar_visible(event)
                    && (self.preferences.tray_all_day || !event.all_day)
            })
            .cloned()
            .collect();
        events.sort_by_key(|event| event.start);
        let today_events: Vec<_> = events
            .iter()
            .filter(|event| event.start.with_timezone(&self.timezone).date_naive() <= today)
            .cloned()
            .collect();
        let shown = if today_events.is_empty() {
            events.into_iter().take(3).collect()
        } else {
            today_events
        };
        div()
            .size_full()
            .p_5()
            .flex()
            .flex_col()
            .gap_3()
            .bg(rgb(0x202020).opacity(0.84))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(div().text_size(px(18.0)).child("Coming up"))
                    .child(
                        button("close-agenda", "×").on_click(|_, window, _| window.remove_window()),
                    ),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(MUTED))
                    .child(format!(
                        "{} · {}",
                        today.format("%A, %d %B"),
                        if self.online {
                            "Local preview"
                        } else {
                            "Offline cache"
                        }
                    )),
            )
            .when(shown.is_empty(), |element| {
                element.child(
                    div()
                        .py_6()
                        .text_color(rgb(MUTED))
                        .child("Nothing coming up in your cached calendar."),
                )
            })
            .children(shown.into_iter().take(6).map(|event| {
                div()
                    .id(SharedString::from(event.event_id.clone()))
                    .p_3()
                    .rounded_lg()
                    .border_l_2()
                    .border_color(rgb(event.color))
                    .bg(rgb(event.color).opacity(0.12))
                    .cursor_pointer()
                    .child(div().text_size(px(12.0)).child(event.title.clone()))
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(11.0))
                            .text_color(rgb(MUTED))
                            .child(
                                event
                                    .start
                                    .with_timezone(&self.timezone)
                                    .format("%a %d · %H:%M")
                                    .to_string(),
                            ),
                    )
                    .on_click(context.listener(move |_, _, window, context| {
                        crate::show_event(event.clone(), context);
                        window.remove_window();
                    }))
            }))
            .child(div().flex_1())
            .child(
                button("open-main", "Open calendar  ↗").on_click(|_, _, context| {
                    context.activate(true);
                    crate::open_main(context);
                }),
            )
    }
}

impl Render for CalendarApp {
    fn render(&mut self, window: &mut Window, context: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .id("calendar-app")
            .track_focus(&self.focus)
            .relative()
            .size_full()
            .flex()
            .text_color(rgb(INK))
            .text_size(px(13.0))
            .bg(rgb(0x191919))
            .font_family("Inter")
            .on_mouse_move(
                context.listener(|app, event: &MouseMoveEvent, window, context| {
                    app.move_gesture(event.position, window, context)
                }),
            )
            .on_mouse_up(
                MouseButton::Left,
                context.listener(|app, _, window, context| app.finish_gesture(window, context)),
            )
            .on_key_down(
                context.listener(|app, event: &KeyDownEvent, window, context| {
                    if app.selected.is_none()
                        && app.gesture.is_none()
                        && !app.activity_open
                        && !app.settings_open
                        && !app.profile_open
                        && !event.keystroke.modifiers.control
                        && !event.keystroke.modifiers.alt
                        && !event.keystroke.modifiers.platform
                    {
                        match event.keystroke.key.as_str() {
                            "w" => app.set_month_view(false, context),
                            "m" => app.set_month_view(true, context),
                            "t" => {
                                app.go_today();
                                context.notify();
                            }
                            _ => (),
                        }
                    }
                    if event.keystroke.key == "escape" {
                        app.view_menu_open = false;
                        app.settings_open = false;
                        app.profile_open = false;
                        if let Some(gesture) = app.gesture.take() {
                            app.events
                                .retain(|event| event.event_id != gesture.event.event_id);
                            if gesture.event.revision >= 0 {
                                app.events.push(gesture.event);
                            }
                        }
                        app.selected = None;
                        if app.agenda_only {
                            window.remove_window();
                        } else {
                            app.focus.focus(window, context);
                            context.notify();
                        }
                    }
                }),
            );
        if self.agenda_only {
            return root.child(self.agenda(context)).into_any_element();
        }
        let first_date = self.date;
        let day_width = ((f32::from(window.viewport_size().width) - self.sidebar_width() - GUTTER)
            / 7.0)
            .max(80.0);
        root = root
            .when(self.sidebar_open, |root| root.child(self.sidebar(context)))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .bg(rgb(0x191919))
                    .child(
                        TitleBar::new()
                            .h(px(42.0))
                            .border_b_0()
                            .bg(rgb(0x191919))
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .justify_end()
                                    .items_center()
                                    .gap_2()
                                    .pr_4()
                                    .when(!self.sidebar_open, |element| {
                                        element.child(
                                            icon_button("sidebar-open", "icons/panel-left.svg")
                                                .on_click(context.listener(
                                                    |app, _, _, context| {
                                                        app.toggle_sidebar(context)
                                                    },
                                                )),
                                        )
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .on_mouse_down(MouseButton::Left, |_, window, _| {
                                                window.start_window_move()
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id("view-selector")
                                            .occlude()
                                            .on_mouse_down(MouseButton::Left, |_, _, context| {
                                                context.stop_propagation()
                                            })
                                            .cursor_pointer()
                                            .on_click(context.listener(|app, _, _, context| {
                                                app.view_menu_open = !app.view_menu_open;
                                                context.notify();
                                            }))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py_1()
                                            .rounded_md()
                                            .bg(rgb(0x272727))
                                            .border_1()
                                            .border_color(rgb(0x343434))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(if self.month_view { "Month" } else { "Week" })
                                            .child(
                                                svg()
                                                    .path("icons/chevron-down.svg")
                                                    .size(px(16.0))
                                                    .text_color(rgb(0xbdbdbd)),
                                            ),
                                    )
                                    .child(
                                        button("today", "Today")
                                            .px_3()
                                            .bg(rgb(0x272727))
                                            .border_1()
                                            .border_color(rgb(0x343434))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .on_click(context.listener(|app, _, _, context| {
                                                app.go_today();
                                                app.horizontal_fraction = 0.0;
                                                context.notify();
                                            })),
                                    )
                                    .child(
                                        icon_button(
                                            "previous",
                                            if self.month_view {
                                                "icons/chevron-up.svg"
                                            } else {
                                                "icons/chevron-left.svg"
                                            },
                                        )
                                        .text_size(px(21.0))
                                        .text_color(rgb(MUTED))
                                        .on_click(
                                            context.listener(|app, _, _, context| {
                                                app.navigate(-7, context)
                                            }),
                                        ),
                                    )
                                    .child(
                                        icon_button(
                                            "next",
                                            if self.month_view {
                                                "icons/chevron-down.svg"
                                            } else {
                                                "icons/chevron-right.svg"
                                            },
                                        )
                                        .text_size(px(21.0))
                                        .text_color(rgb(MUTED))
                                        .on_click(
                                            context.listener(|app, _, _, context| {
                                                app.navigate(7, context)
                                            }),
                                        ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .h(px(40.0))
                            .flex_shrink_0()
                            .px_5()
                            .flex()
                            .items_end()
                            .pb_1()
                            .child(
                                div()
                                    .text_size(px(22.0))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(rgb(0xd4d4d4))
                                    .child(self.date.format("%B %Y").to_string()),
                            ),
                    )
                    .when(!self.month_view, |element| {
                        element
                            .child(
                                div()
                                    .h(px(26.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .border_b_1()
                                    .border_color(rgb(0xffffff).opacity(0.07))
                                    .child(
                                        div()
                                            .w(px(GUTTER))
                                            .flex_shrink_0()
                                            .text_center()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .child("GMT+7"),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .flex_1()
                                            .h_full()
                                            .overflow_hidden()
                                            .on_scroll_wheel(
                                                context.listener(Self::scroll_timeline),
                                            )
                                            .children((0..8).map(|day| {
                                                let date = first_date + Duration::days(day);
                                                let today = date
                                                    == Utc::now()
                                                        .with_timezone(&self.timezone)
                                                        .date_naive();
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .left(px((day as f32
                                                        - self.horizontal_fraction)
                                                        * day_width))
                                                    .h_full()
                                                    .w(px(day_width))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .gap_2()
                                                    .text_color(rgb(if today {
                                                        INK
                                                    } else {
                                                        MUTED
                                                    }))
                                                    .child(date.format("%a").to_string())
                                                    .child(
                                                        div()
                                                            .w(px(20.0))
                                                            .h(px(22.0))
                                                            .rounded_md()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(today, |element| {
                                                                element
                                                                    .bg(rgb(0xf45151))
                                                                    .text_color(rgb(0xffffff))
                                                            })
                                                            .child(date.day().to_string()),
                                                    )
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .h(px(24.0))
                                    .flex_shrink_0()
                                    .flex()
                                    .border_b_1()
                                    .border_color(rgb(0xffffff).opacity(0.07))
                                    .child(
                                        div()
                                            .w(px(GUTTER))
                                            .flex_shrink_0()
                                            .text_size(px(9.0))
                                            .text_color(rgb(MUTED))
                                            .pt_2()
                                            .text_center()
                                            .child("all-day"),
                                    )
                                    .child(
                                        div()
                                            .relative()
                                            .flex_1()
                                            .h_full()
                                            .overflow_hidden()
                                            .on_scroll_wheel(
                                                context.listener(Self::scroll_timeline),
                                            )
                                            .children((0..8).map(|day| {
                                                let date = first_date + Duration::days(day);
                                                div()
                                                    .absolute()
                                                    .top_0()
                                                    .left(px((day as f32
                                                        - self.horizontal_fraction)
                                                        * day_width))
                                                    .h_full()
                                                    .w(px(day_width))
                                                    .overflow_hidden()
                                                    .children(
                                                        self.events
                                                            .iter()
                                                            .filter(|event| {
                                                                event.all_day
                                                                    && event
                                                                        .start
                                                                        .with_timezone(
                                                                            &self.timezone,
                                                                        )
                                                                        .date_naive()
                                                                        <= date
                                                                    && event
                                                                        .end
                                                                        .with_timezone(
                                                                            &self.timezone,
                                                                        )
                                                                        .date_naive()
                                                                        > date
                                                                    && self.calendar_visible(event)
                                                            })
                                                            .map(|event| {
                                                                let event = event.clone();
                                                                div()
                                                        .id(SharedString::from(format!(
                                                            "all-day-{}-{day}",
                                                            event.event_id
                                                        )))
                                                        .m_1()
                                                        .px_2()
                                                        .rounded_md()
                                                        .text_size(px(10.0))
                                                        .bg(rgb(event.color).opacity(0.2))
                                                        .cursor_pointer()
                                                        .child(event.title.clone())
                                                        .on_click(context.listener(
                                                            move |app, _, window, context| {
                                                                app.edit(
                                                                    event.clone(),
                                                                    window,
                                                                    context,
                                                                )
                                                            },
                                                        ))
                                                            }),
                                                    )
                                            })),
                                    ),
                            )
                            .child(self.week_grid(window, context))
                    })
                    .when(self.month_view, |element| {
                        element.child(self.month_grid(context))
                    })
                    .when(
                        self.pending
                            || self.message.contains("not configured")
                            || self.message.contains("failed")
                            || self.message.contains("unavailable"),
                        |element| {
                            element.child(
                                div()
                                    .absolute()
                                    .bottom_3()
                                    .right_3()
                                    .max_w(px(420.0))
                                    .p_3()
                                    .rounded_md()
                                    .bg(rgb(0x303030))
                                    .text_size(px(11.0))
                                    .text_color(rgb(INK))
                                    .child(self.message.clone()),
                            )
                        },
                    ),
            );
        if self.view_menu_open {
            root = root.child(
                div()
                    .absolute()
                    .top(px(42.0))
                    .right(px(230.0))
                    .w(px(150.0))
                    .p_1()
                    .rounded_md()
                    .bg(rgb(0x292929))
                    .border_1()
                    .border_color(rgb(0x404040))
                    .child(button("view-week", "Week                  W").on_click(
                        context.listener(|app, _, _, context| app.set_month_view(false, context)),
                    ))
                    .child(button("view-month", "Month                M").on_click(
                        context.listener(|app, _, _, context| app.set_month_view(true, context)),
                    )),
            );
        }
        if !self.agenda_only {
            let account = self.google_connection.accounts.get(self.active_account);
            let avatar = div()
                .size(px(28.0))
                .rounded_full()
                .overflow_hidden()
                .bg(rgb(0x3c526a))
                .when_some(
                    account.filter(|account| !account.picture.is_empty()),
                    |element, account| {
                        element.child(
                            img(std::path::PathBuf::from(account.picture.clone()))
                                .size_full()
                                .object_fit(ObjectFit::Cover),
                        )
                    },
                )
                .when(
                    account.is_none_or(|account| account.picture.is_empty()),
                    |element| {
                        element.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    account
                                        .and_then(|account| {
                                            account
                                                .name
                                                .chars()
                                                .next()
                                                .or_else(|| account.email.chars().next())
                                        })
                                        .unwrap_or('G')
                                        .to_string(),
                                ),
                        )
                    },
                );
            root = root.child(
                div()
                    .id("profile-toggle")
                    .absolute()
                    .left(px(
                        self.sidebar_width() + if self.sidebar_open { 18.0 } else { 56.0 }
                    ))
                    .top(px(5.0))
                    .size(px(30.0))
                    .occlude()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, context| {
                        context.stop_propagation()
                    })
                    .on_click(context.listener(|app, _, _, context| {
                        app.profile_open = !app.profile_open;
                        context.notify();
                    }))
                    .child(avatar),
            );
        }
        if self.profile_open {
            root = root.child(self.profile_menu(context));
        }
        if self.settings_open {
            root = root.child(self.settings(context));
        }
        if self.selected.is_some() && !self.settings_open {
            root = root.child(self.editor(context));
        }
        if self.activity_open && !self.settings_open {
            root = root.child(self.activity_panel(context));
        }
        root.into_any_element()
    }
}

fn event_surface(color: u32) -> u32 {
    let channel = |shift: u32| (((color >> shift) & 255u32) as f32 * 0.30 + 25.0 * 0.70) as u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}

fn icon_button(identifier: &'static str, path: &'static str) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .size(px(30.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .cursor_pointer()
        .text_color(rgb(MUTED))
        .hover(|style| style.bg(rgb(0xffffff).opacity(0.07)))
        .on_click(move |_, _, _| calendar_telemetry::track("control_clicked", identifier))
        .child(svg().path(path).size(px(18.0)).text_color(rgb(0xbdbdbd)))
}

fn button(identifier: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .on_click(move |_, _, _| calendar_telemetry::track("control_clicked", identifier))
        .flex()
        .items_center()
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .text_size(px(12.0))
        .hover(|style| style.bg(rgb(0xffffff).opacity(0.07)))
        .child(label)
}
fn calendar_toggle(
    identifier: SharedString,
    title: SharedString,
    color: u32,
    visible: bool,
) -> Stateful<Div> {
    div()
        .id(identifier)
        .occlude()
        .on_mouse_down(MouseButton::Left, |_, _, context| {
            context.stop_propagation()
        })
        .h(px(30.0))
        .px_2()
        .flex()
        .items_center()
        .gap_3()
        .rounded_md()
        .cursor_pointer()
        .hover(|style| style.bg(rgb(0x303030)))
        .child(
            div()
                .size(px(12.0))
                .rounded_sm()
                .bg(rgb(if visible { color } else { 0x555555 })),
        )
        .child(
            div()
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(if visible { 0xcccccc } else { MUTED }))
                .truncate()
                .child(title),
        )
}
fn field(label: &'static str, input: impl IntoElement) -> Div {
    div()
        .mb_3()
        .child(
            div()
                .text_size(px(9.0))
                .text_color(rgb(MUTED))
                .mb_1()
                .child(label),
        )
        .child(input)
}

#[cfg(all(test, feature = "ui-tests"))]
mod tests {
    use super::*;
    use core::prelude::v1::test;
    #[gpui::test]
    fn activity_scroll_does_not_move_calendar(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let (app, context) =
            context.add_window_view(|window, context| CalendarApp::new(window, context, false));
        app.update(context, |app, _| {
            app.activity_open = true;
            app.backend_logs = (0..100)
                .map(|index| calendar_telemetry::LogEntry {
                    timestamp: format!("2026-10-15T10:00:{:02}Z", index % 60),
                    component: "backend".into(),
                    level: "INFO".into(),
                    action: "test_activity".into(),
                    details: Default::default(),
                })
                .collect();
        });
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        let position = context.debug_bounds("activity-entries").unwrap().center();
        let before = app.read_with(context, |app, _| {
            (app.scroll.offset(), app.date, app.horizontal_fraction)
        });
        context.simulate_mouse_move(position, None, Modifiers::default());
        context.simulate_event(ScrollWheelEvent {
            position,
            delta: ScrollDelta::Pixels(point(px(0.0), px(-80.0))),
            ..Default::default()
        });
        app.read_with(context, |app, _| {
            assert!(app.activity_scroll.offset().y < px(0.0));
            assert_eq!(
                (app.scroll.offset(), app.date, app.horizontal_fraction),
                before
            );
        });
        for delta in [
            point(px(-80.0), px(0.0)),
            point(px(0.0), px(100000.0)),
            point(px(0.0), px(80.0)),
        ] {
            context.simulate_event(ScrollWheelEvent {
                position,
                delta: ScrollDelta::Pixels(delta),
                ..Default::default()
            });
        }
        app.read_with(context, |app, _| {
            assert_eq!(
                (app.scroll.offset(), app.date, app.horizontal_fraction),
                before
            )
        });
    }

    #[gpui::test]
    fn dropped_event_stays_optimistic_until_success_or_failure(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        let (sender, requests) = std::sync::mpsc::channel();
        let (updates, receiver) = std::sync::mpsc::channel();
        handle
            .update(context, |app, window, context| {
                app.connection = service::Connection { sender, receiver };
                app.online = true;
                let mut original = app.new_event(app.date, 540);
                original.revision = 1;
                app.events.push(original.clone());
                app.begin_gesture(
                    original.clone(),
                    GestureKind::Move,
                    point(px(400.0), px(300.0)),
                    context,
                );
                let day_width =
                    (f32::from(window.viewport_size().width) - app.sidebar_width() - GUTTER) / 7.0;
                app.move_gesture(point(px(400.0 + day_width), px(300.0)), window, context);
                app.finish_gesture(window, context);
                assert!(app.pending);
                assert_eq!(app.events[0].start, original.start + Duration::days(1));
                assert!(matches!(requests.try_recv().unwrap(), Request::Save(_)));
                updates
                    .send(Update::Snapshot(vec![original.clone()], true))
                    .unwrap();
                app.poll(context);
                assert_eq!(app.events[0].start, original.start + Duration::days(1));
                let mut saved = app.events[0].clone();
                saved.revision = 2;
                updates.send(Update::Saved(saved.clone())).unwrap();
                app.poll(context);
                assert!(!app.pending);
                assert_eq!(app.events[0].start, saved.start);
                assert_eq!(app.events[0].revision, 2);
                let mut moved = saved.clone();
                moved.start += Duration::days(1);
                moved.end += Duration::days(1);
                app.commit(moved.clone(), context);
                assert_eq!(app.events[0].start, moved.start);
                updates
                    .send(Update::Failed("test rejection".into()))
                    .unwrap();
                app.poll(context);
                assert_eq!(app.events[0].start, saved.start);
                assert!(!app.pending);
                assert!(app.message.contains("failed"));
                drop(requests);
                app.commit(moved, context);
                assert_eq!(app.events[0].start, saved.start);
                assert!(!app.pending);
            })
            .unwrap();
    }

    #[gpui::test]
    fn today_and_month_grid_follow_week_start_preference(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        handle
            .update(context, |app, _, _| {
                let thursday = NaiveDate::from_ymd_opt(2026, 10, 15).unwrap();
                assert_eq!(week_start(thursday, false).day(), 12);
                assert_eq!(week_start(thursday, true).day(), 11);
                app.preferences.sunday_first = false;
                app.go_today();
                assert_eq!(app.date.weekday(), chrono::Weekday::Mon);
                app.preferences.sunday_first = true;
                app.go_today();
                assert_eq!(app.date.weekday(), chrono::Weekday::Sun);
                assert_eq!(app.display_month_cells()[0].weekday(), chrono::Weekday::Sun);
            })
            .unwrap();
    }

    #[gpui::test]
    fn settings_and_event_popover_do_not_scroll_the_timeline(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let (app, context) =
            context.add_window_view(|window, context| CalendarApp::new(window, context, false));
        for settings in [false, true] {
            app.update(context, |app, context| {
                let mut event = app.new_event(app.date, 540);
                event.revision = 0;
                app.selected = Some(event);
                app.settings_open = settings;
                context.notify();
            });
            context.draw(
                point(px(0.0), px(0.0)),
                size(px(1050.0), px(680.0)),
                |_, _| app.clone().into_any_element(),
            );
            let before = app.read_with(context, |app, _| (app.date, app.scroll.offset()));
            context.simulate_mouse_move(point(px(800.0), px(350.0)), None, Modifiers::default());
            context.simulate_event(ScrollWheelEvent {
                position: point(px(800.0), px(350.0)),
                delta: ScrollDelta::Pixels(point(px(-70.0), px(-70.0))),
                ..Default::default()
            });
            app.read_with(context, |app, _| {
                assert_eq!(app.date, before.0);
                assert_eq!(app.scroll.offset(), before.1);
            });
        }
    }

    #[gpui::test]
    fn sidebar_buttons_close_and_reopen_with_pointer_clicks(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let (app, context) =
            context.add_window_view(|window, context| CalendarApp::new(window, context, false));
        for _ in 0..2 {
            context.draw(
                point(px(0.0), px(0.0)),
                size(px(1000.0), px(600.0)),
                |_, _| app.clone().into_any_element(),
            );
            context.simulate_click(point(px(57.0), px(27.0)), Modifiers::default());
            app.read_with(context, |app, _| assert!(!app.sidebar_open));
            context.draw(
                point(px(0.0), px(0.0)),
                size(px(1000.0), px(600.0)),
                |_, _| app.clone().into_any_element(),
            );
            context.simulate_click(point(px(27.0), px(21.0)), Modifiers::default());
            app.read_with(context, |app, _| assert!(app.sidebar_open));
        }
    }

    #[gpui::test]
    fn view_shortcuts_and_month_navigation(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let (app, context) =
            context.add_window_view(|window, context| CalendarApp::new(window, context, false));
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        context.simulate_keystrokes("m");
        app.update(context, |app, context| {
            assert!(app.month_view);
            app.date = NaiveDate::from_ymd_opt(2026, 12, 31).unwrap();
            app.navigate(7, context);
            assert_eq!(app.date, NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
            app.navigate(-7, context);
            assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
            assert!(app.date_visible(NaiveDate::from_ymd_opt(2026, 12, 31).unwrap()));
            assert!(!app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()));
            app.toggle_sidebar(context);
            assert_eq!(app.sidebar_width(), 0.0);
        });
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        context.simulate_keystrokes("w");
        app.update(context, |app, _| {
            assert!(!app.month_view);
            app.selected = Some(app.new_event(app.date, 540));
        });
        context.simulate_keystrokes("m");
        app.read_with(context, |app, _| assert!(!app.month_view));
    }

    #[gpui::test]
    fn sidebar_highlights_seven_dates_without_counting_clipped_edges(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        handle
            .update(context, |app, _, _| {
                app.date = NaiveDate::from_ymd_opt(2026, 10, 12).unwrap();
                for fraction in [0.0, f32::EPSILON, 0.01, 0.49, 0.5, 0.99] {
                    app.horizontal_fraction = fraction;
                    let highlighted: Vec<_> = (0..9)
                        .map(|day| app.date + Duration::days(day))
                        .filter(|date| app.date_visible(*date))
                        .collect();
                    let first_day = if fraction < 0.5 { 12 } else { 13 };
                    assert_eq!(highlighted.len(), 7);
                    assert_eq!(highlighted[0].day(), first_day);
                    assert_eq!(highlighted[6].day(), first_day + 6);
                }
            })
            .unwrap();
    }

    #[gpui::test]
    fn horizontal_scroll_tracks_visible_dates_across_year_boundary(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        handle
            .update(context, |app, window, context| {
                app.date = NaiveDate::from_ymd_opt(2026, 12, 28).unwrap();
                let original_vertical = app.scroll.offset().y;
                app.scroll_dates(-150.0, 100.0, context);
                assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 29).unwrap());
                assert_eq!(app.horizontal_fraction, 0.5);
                assert!(app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()));
                assert!(!app.date_visible(NaiveDate::from_ymd_opt(2026, 12, 28).unwrap()));
                app.scroll_dates(50.0, 100.0, context);
                assert_eq!(app.horizontal_fraction, 0.0);
                assert!(!app.date_visible(NaiveDate::from_ymd_opt(2027, 1, 5).unwrap()));
                assert_eq!(app.scroll.offset().y, original_vertical);
                app.scroll_dates(200.0, 100.0, context);
                assert_eq!(app.date, NaiveDate::from_ymd_opt(2026, 12, 27).unwrap());
                let event = app.new_event(app.date + Duration::days(2), 540);
                assert_eq!(
                    event.start.with_timezone(&app.timezone).date_naive(),
                    NaiveDate::from_ymd_opt(2026, 12, 29).unwrap()
                );
                app.scroll_timeline(
                    &ScrollWheelEvent {
                        delta: ScrollDelta::Pixels(point(px(0.0), px(-60.0))),
                        ..Default::default()
                    },
                    window,
                    context,
                );
                assert_eq!(app.horizontal_fraction, 0.0);
                app.scroll_timeline(
                    &ScrollWheelEvent {
                        delta: ScrollDelta::Pixels(point(px(0.0), px(-60.0))),
                        modifiers: Modifiers {
                            shift: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    window,
                    context,
                );
                assert!(app.horizontal_fraction > 0.0);
                assert_eq!(app.scroll.offset().y, original_vertical);
            })
            .unwrap();
    }

    #[gpui::test]
    fn timeline_dispatch_keeps_horizontal_and_vertical_scroll_independent(
        context: &mut TestAppContext,
    ) {
        context.update(gpui_kit::init);
        let (app, context) =
            context.add_window_view(|window, context| CalendarApp::new(window, context, false));
        context.draw(
            point(px(0.0), px(0.0)),
            size(px(1000.0), px(600.0)),
            |_, _| app.clone().into_any_element(),
        );
        let before = app.read_with(context, |app, _| (app.date, app.scroll.offset().y));
        context.simulate_mouse_move(point(px(400.0), px(300.0)), None, Modifiers::default());
        context.simulate_event(ScrollWheelEvent {
            position: point(px(400.0), px(300.0)),
            delta: ScrollDelta::Pixels(point(px(-50.0), px(0.0))),
            ..Default::default()
        });
        app.read_with(context, |app, _| {
            assert!(app.horizontal_fraction > 0.0 || app.date > before.0);
            assert_eq!(app.scroll.offset().y, before.1);
        });
        let horizontal_before =
            app.read_with(context, |app, _| (app.date, app.horizontal_fraction));
        context.simulate_event(ScrollWheelEvent {
            position: point(px(400.0), px(300.0)),
            delta: ScrollDelta::Pixels(point(px(0.0), px(50.0))),
            ..Default::default()
        });
        app.read_with(context, |app, _| {
            assert_eq!((app.date, app.horizontal_fraction), horizontal_before);
            assert!(
                app.scroll.offset().y > before.1,
                "before={:?} after={:?}",
                before.1,
                app.scroll.offset().y
            );
        });
    }

    #[gpui::test]
    fn offline_edits_leave_events_unchanged(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        handle
            .update(context, |app, _, context| {
                let event = app.new_event(app.date, 540);
                app.commit(event, context);
                assert!(!app.pending);
                assert!(app.events.is_empty());
                assert!(app.message.contains("Offline"));
            })
            .unwrap();
    }
    #[gpui::test]
    fn move_and_resize_preserve_valid_ranges(context: &mut TestAppContext) {
        context.update(gpui_kit::init);
        let handle = context.add_window(|window, context| CalendarApp::new(window, context, false));
        handle
            .update(context, |app, window, context| {
                app.online = true;
                let event = app.new_event(app.date, 540);
                app.events.push(event.clone());
                app.begin_gesture(
                    event.clone(),
                    GestureKind::Move,
                    point(px(300.0), px(300.0)),
                    context,
                );
                app.move_gesture(point(px(300.0), px(300.0 + HOUR)), window, context);
                assert_eq!(app.events[0].start, event.start + Duration::hours(1));
                assert_eq!(app.events[0].end - app.events[0].start, Duration::hours(1));
                app.begin_gesture(
                    event.clone(),
                    GestureKind::End,
                    point(px(300.0), px(300.0)),
                    context,
                );
                app.move_gesture(point(px(300.0), px(100.0)), window, context);
                assert_eq!(
                    app.events[0].end - app.events[0].start,
                    Duration::minutes(15)
                );
            })
            .unwrap();
    }
}

fn week_start(date: NaiveDate, sunday_first: bool) -> NaiveDate {
    let offset = if sunday_first {
        date.weekday().num_days_from_sunday()
    } else {
        date.weekday().num_days_from_monday()
    };
    date - Duration::days(i64::from(offset))
}
impl CalendarApp {
    fn profile_menu(&self, context: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("profile-menu")
            .absolute()
            .top(px(42.0))
            .left(px(
                self.sidebar_width() + if self.sidebar_open { 18.0 } else { 56.0 }
            ))
            .w(px(280.0))
            .p_3()
            .rounded_lg()
            .bg(rgb(0x282828))
            .border_1()
            .border_color(rgb(0x393939))
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, context| {
                context.stop_propagation()
            })
            .on_scroll_wheel(|_, _, context| context.stop_propagation())
            .children(
                self.google_connection
                    .accounts
                    .iter()
                    .enumerate()
                    .map(|(index, account)| {
                        div()
                            .id(SharedString::from(format!("profile-{index}")))
                            .p_2()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x343434)))
                            .child(div().font_weight(FontWeight::SEMIBOLD).child(
                                if account.name.is_empty() {
                                    "Google account".into()
                                } else {
                                    account.name.clone()
                                },
                            ))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(rgb(MUTED))
                                    .truncate()
                                    .child(account.email.clone()),
                            )
                            .on_click(context.listener(move |app, _, _, context| {
                                app.active_account = index;
                                context.notify();
                            }))
                    }),
            )
            .child(
                button("open-settings", "Settings")
                    .w_full()
                    .on_click(context.listener(|app, _, _, context| {
                        app.profile_open = false;
                        app.settings_open = true;
                        context.notify();
                    })),
            )
            .when(!self.google_connection.accounts.is_empty(), |element| {
                element.child(
                    button("logout", "Log out of this account")
                        .w_full()
                        .on_click(context.listener(|app, _, _, context| {
                            if let Some(account) =
                                app.google_connection.accounts.get(app.active_account)
                            {
                                let _ = app
                                    .connection
                                    .sender
                                    .send(Request::DisconnectGoogle(Some(account.id.clone())));
                            }
                            app.profile_open = false;
                            app.active_account = 0;
                            context.notify();
                        })),
                )
            })
    }

    fn settings(&self, context: &mut Context<Self>) -> impl IntoElement {
        let mut content = div()
            .flex_1()
            .min_w_0()
            .p_6()
            .id("settings-content")
            .overflow_y_scroll();
        content = match self.settings_section.as_str() {
            "profile" => content.child(div().text_size(px(22.0)).font_weight(FontWeight::SEMIBOLD).mb_6().child("Profile"))
                .children(self.google_connection.accounts.iter().map(|account| {
                    div().mb_6().flex().gap_4().items_center()
                        .child(div().size(px(64.0)).rounded_full().overflow_hidden().bg(rgb(0x3c526a))
                            .when(!account.picture.is_empty(), |element| element.child(img(std::path::PathBuf::from(account.picture.clone())).size_full().object_fit(ObjectFit::Cover))))
                        .child(div().child(account.name.clone()).child(div().text_color(rgb(MUTED)).child(account.email.clone())))
                })),
            "tray" => content.child(div().text_size(px(22.0)).mb_6().child("Tray calendar"))
                .child(div().mb_4().text_color(rgb(MUTED)).child("Click the tray icon to see today's schedule and upcoming events."))
                .child(button("tray-all-day",if self.preferences.tray_all_day { "All-day events: On" } else { "All-day events: Off" })
                    .on_click(context.listener(|app,_,_,context| { app.preferences.tray_all_day = !app.preferences.tray_all_day; app.preferences.save(); context.notify(); })))
                .child(div().mt_4().text_color(rgb(MUTED)).child("Changes apply when the tray calendar is next opened.")),
            "accounts" => content.child(div().text_size(px(22.0)).mb_4().child("Google accounts"))
                .child(div().text_size(px(12.0)).mb_4().text_color(rgb(MUTED)).child("Connect accounts and approve calendar access. Changes save directly to Google."))
                .children(self.google_connection.accounts.iter().enumerate().map(|(index, account)| {
                    let reconnect_id = account.id.clone();
                    let disconnect_id = account.id.clone();
                    div().mb_4().p_4().border_1().border_color(rgb(0x393939)).rounded_lg()
                        .child(div().font_weight(FontWeight::SEMIBOLD).child(account.email.clone()))
                        .child(div().mt_2().text_size(px(12.0)).text_color(rgb(MUTED)).child(if account.connecting { "Finish sign-in in your browser.".into() } else if account.write_access { "Calendar access enabled".into() } else { account.message.clone() }))
                        .child(div().mt_3().children(account.calendars.iter().map(|calendar| div().py_1().text_size(px(12.0)).child(calendar.summary.clone()))))
                        .child(div().flex().gap_3().mt_3()
                            .child(div().id(SharedString::from(format!("reconnect-{index}"))).px_3().py_2().rounded_md().bg(rgb(0x343434)).cursor_pointer()
                                .child(if account.connecting { "Waiting for Google…" } else if account.write_access { "Reconnect" } else { "Approve calendar access" })
                                .on_click(context.listener(move |app,_,_,context| {
                                    let _ = app.connection.sender.send(Request::ConnectGoogle(Some(reconnect_id.clone())));
                                    context.notify();
                                })))
                            .child(div().id(SharedString::from(format!("disconnect-{index}"))).px_3().py_2().rounded_md().cursor_pointer().text_color(rgb(0xe99a91))
                                .child(if account.connecting { "Cancel sign-in" } else { "Disconnect" })
                                .on_click(context.listener(move |app,_,_,context| {
                                    let _ = app.connection.sender.send(Request::DisconnectGoogle(Some(disconnect_id.clone())));
                                    context.notify();
                                }))))
                }))
                .child(button("add-google","+ Add Google account").on_click(context.listener(|app,_,_,context| {
                    let _ = app.connection.sender.send(Request::ConnectGoogle(None)); context.notify();
                })))
                .child(div().mt_3().text_size(px(12.0)).text_color(rgb(MUTED)).child(self.google_connection.message.clone()))
                .when_some(self.sync_error.clone(), |element,error| element.child(div().mt_3().text_color(rgb(0xe99a91)).child(error))),
            _ => content.child(div().text_size(px(22.0)).mb_6().font_weight(FontWeight::SEMIBOLD).child("Calendar view"))
                .child(div().mb_2().child("Start week on"))
                .child(button("week-start",if self.preferences.sunday_first { "Sunday   ▾" } else { "Monday   ▾" })
                    .w(px(220.0)).bg(rgb(0x303030)).on_click(context.listener(|app,_,_,context| {
                        app.preferences.sunday_first = !app.preferences.sunday_first;
                        app.preferences.save(); app.go_today(); context.notify();
                    })))
                .child(div().mt_6().mb_2().text_size(px(20.0)).child("Calendar navigation"))
                .child(div().text_color(rgb(MUTED)).child("T — Current week    W — Week view    M — Month view"))
                .child(div().mt_4().text_color(rgb(MUTED)).child("Scroll horizontally to move freely between days. Shift + wheel works with a mouse."))
                .child(div().mt_6().text_color(rgb(MUTED)).child("Display time zone: Asia/Jakarta · GMT+7")),
        };
        div()
            .id("settings-overlay")
            .absolute()
            .inset_0()
            .occlude()
            .bg(rgb(0x000000).opacity(0.35))
            .on_scroll_wheel(|_, _, context| context.stop_propagation())
            .on_mouse_down(MouseButton::Left, |_, _, context| {
                context.stop_propagation()
            })
            .child(
                div()
                    .absolute()
                    .left(px(36.0))
                    .right(px(36.0))
                    .top(px(20.0))
                    .bottom(px(20.0))
                    .flex()
                    .rounded_xl()
                    .overflow_hidden()
                    .bg(rgb(0x242424))
                    .border_1()
                    .border_color(rgb(0x393939))
                    .shadow_lg()
                    .child(
                        div()
                            .w(px(235.0))
                            .flex_shrink_0()
                            .p_3()
                            .bg(rgb(0x202020))
                            .child(
                                div()
                                    .p_2()
                                    .text_size(px(12.0))
                                    .text_color(rgb(MUTED))
                                    .child("Account"),
                            )
                            .children(
                                [
                                    ("general", "General"),
                                    ("profile", "Profile"),
                                    ("tray", "Tray calendar"),
                                    ("accounts", "Google accounts"),
                                ]
                                .into_iter()
                                .map(|(section, label)| {
                                    button(section, label)
                                        .w_full()
                                        .mb_1()
                                        .when(self.settings_section == section, |element| {
                                            element.bg(rgb(0x303030))
                                        })
                                        .on_click(context.listener(move |app, _, _, context| {
                                            app.settings_section = section.into();
                                            context.notify();
                                        }))
                                }),
                            ),
                    )
                    .child(content)
                    .child(
                        button("close-settings", "×")
                            .absolute()
                            .top(px(8.0))
                            .right(px(8.0))
                            .on_click(context.listener(|app, _, window, context| {
                                app.settings_open = false;
                                app.focus.focus(window, context);
                                context.notify();
                            })),
                    ),
            )
    }
}
