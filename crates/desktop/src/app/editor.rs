//! The create and edit form. It keeps its draft in the running process until Google accepts it.

use super::*;
use crate::service::snapshot::CalendarView;
use calendar_core::resolve_local;
use gpui_kit::component::input::{Input, InputState};
use writes::Action;

pub(super) struct Editor {
    /// The event as read; for a create, the draft (its `event_id` is replaced on every attempt).
    pub base: CalendarEvent,
    pub create: bool,
    calendar_key: String,
    all_day: bool,
    pub(super) title: Entity<InputState>,
    date: Entity<InputState>,
    start: Entity<InputState>,
    end: Entity<InputState>,
    end_date: Entity<InputState>,
    location: Entity<InputState>,
    description: Entity<InputState>,
    /// Why the last attempt did not finish, or the saving state.
    pub status: Option<String>,
    /// The operation this form is waiting for; the form cannot be saved again meanwhile.
    pub in_flight: Option<String>,
}

impl CalendarApp {
    /// A new event in the first writable calendar, or why there is none.
    pub(super) fn draft(
        &self,
        date: NaiveDate,
        minute: u32,
        minutes: i64,
    ) -> Result<CalendarEvent, String> {
        let Some(calendar) = self.writable_calendars().next() else {
            return Err(
                "Connect a Google account with calendar editing access in Settings to create events."
                    .into(),
            );
        };
        let start = resolve_local(date, minute, self.timezone)?;

        Ok(CalendarEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            calendar_id: calendar.key.clone(),
            title: String::new(),
            start,
            end: start + Duration::minutes(minutes),
            timezone: self.timezone.name().into(),
            all_day: false,
            description: String::new(),
            location: String::new(),
            color: calendar.color,
            revision: -1,
            read_only: false,
        })
    }

    /// The New button: the next full hour today.
    pub(super) fn new_event(&mut self, window: &mut Window, context: &mut Context<Self>) {
        let now = Utc::now().with_timezone(&self.timezone);
        let minute = ((now.hour() + 1) * 60).min(22 * 60);
        match self.draft(now.date_naive(), minute, 60) {
            Ok(draft) => self.open_editor(draft, true, window, context),
            Err(reason) => {
                self.write_notice = Some(reason);
                context.notify();
            }
        }
    }

    pub(super) fn open_editor(
        &mut self,
        event: CalendarEvent,
        create: bool,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
        let start = event.start.with_timezone(&self.timezone);
        let last = if event.all_day {
            event.end.with_timezone(&self.timezone) - Duration::days(1)
        } else {
            event.end.with_timezone(&self.timezone)
        };
        let input = |placeholder: &'static str,
                     value: String,
                     window: &mut Window,
                     context: &mut Context<Self>| {
            context.new(|context| {
                let mut state = InputState::new(window, context).placeholder(placeholder);
                state.set_value(value, window, context);
                state
            })
        };
        self.editor = Some(Editor {
            create,
            calendar_key: event.calendar_id.clone(),
            all_day: event.all_day,
            title: input("Event title", event.title.clone(), window, context),
            date: input(
                "YYYY-MM-DD",
                start.format("%Y-%m-%d").to_string(),
                window,
                context,
            ),
            start: input("09:00", start.format("%H:%M").to_string(), window, context),
            end: input("10:00", last.format("%H:%M").to_string(), window, context),
            end_date: input(
                "YYYY-MM-DD",
                last.format("%Y-%m-%d").to_string(),
                window,
                context,
            ),
            location: input("Location", event.location.clone(), window, context),
            description: input("Description", event.description.clone(), window, context),
            status: None,
            in_flight: None,
            base: event.clone(),
        });
        self.selected = Some(event);
        context.notify();
    }

    pub(super) fn close_editor(&mut self) {
        self.editor = None;
        self.selected = None;
    }

    pub(super) fn delete_event(&mut self, event: CalendarEvent, context: &mut Context<Self>) {
        match self.submit(Action::Delete(event), context) {
            Ok(_) => self.close_editor(),
            Err(reason) => self.write_notice = Some(reason),
        }
        context.notify();
    }

    fn form_event(&self, editor: &Editor, context: &App) -> Result<CalendarEvent, String> {
        let text = |input: &Entity<InputState>| input.read(context).value().to_string();
        let date: NaiveDate = text(&editor.date)
            .trim()
            .parse()
            .map_err(|_| "Use YYYY-MM-DD for the date.")?;
        let end_date: NaiveDate = text(&editor.end_date)
            .trim()
            .parse()
            .map_err(|_| "Use YYYY-MM-DD for the end date.")?;
        let minutes = |value: String| -> Result<u32, String> {
            let time = chrono::NaiveTime::parse_from_str(value.trim(), "%H:%M")
                .map_err(|_| "Use HH:MM for the time.")?;
            Ok(time.hour() * 60 + time.minute())
        };
        let mut event = editor.base.clone();
        if editor.all_day {
            // The form shows the last day; Google wants the exclusive end.
            event.start = resolve_local(date, 0, self.timezone)?;
            event.end = resolve_local(end_date.max(date) + Duration::days(1), 0, self.timezone)?;
        } else {
            event.start = resolve_local(date, minutes(text(&editor.start))?, self.timezone)?;
            event.end = resolve_local(end_date, minutes(text(&editor.end))?, self.timezone)?;
        }
        event.all_day = editor.all_day;
        event.title = text(&editor.title).trim().to_owned();
        event.location = text(&editor.location);
        event.description = text(&editor.description);
        if editor.create {
            // Every attempt is a new action: a new draft id, so a retry never reuses an old one.
            event.event_id = uuid::Uuid::new_v4().to_string();
            event.calendar_id = editor.calendar_key.clone();
            if let Some(calendar) = self
                .writable_calendars()
                .find(|c| c.key == event.calendar_id)
            {
                event.color = calendar.color;
            }
        }

        Ok(event)
    }

    pub(super) fn save_editor(&mut self, context: &mut Context<Self>) {
        let Some(editor) = &self.editor else {
            return;
        };
        if editor.in_flight.is_some() {
            return;
        }
        let create = editor.create;
        let result = self.form_event(editor, context).and_then(|event| {
            self.submit(
                if create {
                    Action::Create(event)
                } else {
                    Action::Update(event)
                },
                context,
            )
        });
        if let Some(editor) = &mut self.editor {
            match result {
                Ok(operation_id) => {
                    editor.in_flight = Some(operation_id);
                    editor.status = Some("Saving…".into());
                }
                Err(reason) => editor.status = Some(reason),
            }
        }
        context.notify();
    }

    pub(super) fn editor_view(&self, context: &mut Context<Self>) -> impl IntoElement {
        let editor = self.editor.as_ref().unwrap();
        let saving = editor.in_flight.is_some();
        let calendars: Vec<&CalendarView> = self.writable_calendars().collect();
        let toggle =
            |label: &'static str| button(label, label).h(px(32.0)).bg(rgb(FIELD)).rounded_lg();

        popover(
            div()
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .mb_3()
                        .child(if editor.create {
                            "New event"
                        } else {
                            "Edit event"
                        })
                        .child(button("close-editor", "×").on_click(context.listener(
                            |app, _, window, context| {
                                app.close_editor();
                                app.focus.focus(window, context);
                                context.notify();
                            },
                        ))),
                )
                .child(field("TITLE", Input::new(&editor.title)))
                .child(
                    div()
                        .flex()
                        .gap_3()
                        .child(
                            div()
                                .flex_1()
                                .child(field("START DATE", Input::new(&editor.date))),
                        )
                        .child(
                            div()
                                .flex_1()
                                .child(field("END DATE", Input::new(&editor.end_date))),
                        ),
                )
                .when(!editor.all_day, |element| {
                    element.child(
                        div()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .flex_1()
                                    .child(field("START", Input::new(&editor.start))),
                            )
                            .child(div().flex_1().child(field("END", Input::new(&editor.end)))),
                    )
                })
                .child(
                    toggle(if editor.all_day {
                        "☑  All day"
                    } else {
                        "□  All day"
                    })
                    .mb_3()
                    .on_click(context.listener(|app, _, _, context| {
                        if let Some(editor) = &mut app.editor {
                            editor.all_day = !editor.all_day;
                        }
                        context.notify();
                    })),
                )
                .child(field("LOCATION", Input::new(&editor.location)))
                .child(field("NOTES", Input::new(&editor.description)))
                .child(if editor.create {
                    field(
                        "CALENDAR",
                        div().children(calendars.iter().map(|calendar| {
                            let key = calendar.key.clone();
                            let chosen = calendar.key == editor.calendar_key;
                            calendar_toggle(
                                SharedString::from(format!("pick-{key}")),
                                calendar.summary.clone().into(),
                                calendar.color,
                                chosen,
                            )
                            .on_click(context.listener(
                                move |app, _, _, context| {
                                    if let Some(editor) = &mut app.editor {
                                        editor.calendar_key = key.clone();
                                    }
                                    context.notify();
                                },
                            ))
                        })),
                    )
                } else {
                    field(
                        "CALENDAR",
                        div().child(self.calendar_label(&editor.base.calendar_id)),
                    )
                })
                .when(
                    self.snapshot.recurring.contains(&editor.base.event_id),
                    |element| element.child(note(RECURRING_NOTE, MUTED)),
                )
                .when_some(editor.status.clone(), |element, status| {
                    element.child(note(status, if saving { MUTED } else { DANGER }))
                })
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .mt_2()
                        .when(!editor.create, |element| {
                            let event = editor.base.clone();
                            element.child(
                                button("delete-event", "Delete")
                                    .h(px(32.0))
                                    .text_color(rgb(DANGER))
                                    .mr_auto()
                                    .on_click(context.listener(move |app, _, _, context| {
                                        app.delete_event(event.clone(), context)
                                    })),
                            )
                        })
                        .child(
                            button("save-event", if saving { "Saving…" } else { "Save" })
                                .h(px(32.0))
                                .ml_auto()
                                .bg(rgb(ACCENT))
                                .text_color(rgb(ON_ACCENT))
                                .rounded_lg()
                                .when(saving, |element| element.opacity(0.6))
                                .on_click(
                                    context.listener(|app, _, _, context| app.save_editor(context)),
                                ),
                        ),
                ),
        )
    }
}
