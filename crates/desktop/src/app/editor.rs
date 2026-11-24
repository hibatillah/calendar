use super::*;

impl CalendarApp {
    pub(super) fn new_event(&self, date: NaiveDate, minute: u32) -> CalendarEvent {
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

    pub(super) fn edit(
        &mut self,
        event: CalendarEvent,
        window: &mut Window,
        context: &mut Context<Self>,
    ) {
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

    pub(super) fn rollback_pending_change(&mut self) {
        if let Some((event_id, original)) = self.pending_change.take() {
            self.events.retain(|event| event.event_id != event_id);
            if let Some(original) = original {
                self.events.push(original);
            }
        }
        self.pending = false;
    }

    pub(super) fn commit(&mut self, event: CalendarEvent, context: &mut Context<Self>) {
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

    pub(super) fn save_form(&mut self, context: &mut Context<Self>) {
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

    pub(super) fn editor(&self, context: &mut Context<Self>) -> impl IntoElement {
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
}
