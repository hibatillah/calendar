use super::*;

impl CalendarApp {
    pub(super) fn sidebar(&self, context: &mut Context<Self>) -> Div {
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

    pub(super) fn activity_panel(&self, context: &mut Context<Self>) -> impl IntoElement {
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
}
