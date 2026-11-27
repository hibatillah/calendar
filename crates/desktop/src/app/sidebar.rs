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
                        self.snapshot
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
                                            let calendar_key = calendar.key.clone();
                                            calendar_toggle(
                                                SharedString::from(format!(
                                                    "calendar-{}",
                                                    calendar.key
                                                )),
                                                SharedString::from(calendar.summary.clone()),
                                                calendar.color,
                                                !self.hidden_calendars.contains(&calendar.key),
                                            )
                                            .on_click(
                                                context.listener(move |app, _, _, context| {
                                                    if !app.hidden_calendars.remove(&calendar_key) {
                                                        app.hidden_calendars
                                                            .insert(calendar_key.clone());
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
    }
}
