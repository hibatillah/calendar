use super::*;

impl CalendarApp {
    pub(super) fn profile_menu(&self, context: &mut Context<Self>) -> impl IntoElement {
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
                self.snapshot
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
            .when(!self.snapshot.accounts.is_empty(), |element| {
                element.child(
                    button("logout", "Log out of this account")
                        .w_full()
                        .on_click(context.listener(|app, _, _, context| {
                            if let Some(account) = app.snapshot.accounts.get(app.active_account) {
                                app.handle.disconnect(account.account_id.clone());
                            }
                            app.profile_open = false;
                            app.active_account = 0;
                            context.notify();
                        })),
                )
            })
    }

    /// Re-aligns the shown week to the new first day, then remembers the choice.
    pub(super) fn set_week_start(&mut self, sunday_first: bool, context: &mut Context<Self>) {
        if self.preferences.sunday_first == sunday_first {
            return;
        }
        self.preferences.sunday_first = sunday_first;
        if !self.month_view {
            self.date = week_start(self.date, sunday_first);
        }
        self.horizontal_fraction = 0.0;
        self.preferences.save();
        context.notify();
    }

    pub(super) fn set_tray_all_day(&mut self, tray_all_day: bool, context: &mut Context<Self>) {
        self.preferences.tray_all_day = tray_all_day;
        self.preferences.save();
        context.notify();
    }

    pub(super) fn choice(
        identifier: &'static str,
        label: &'static str,
        selected: bool,
    ) -> Stateful<Div> {
        button(identifier, label)
            .px_3()
            .bg(rgb(if selected { FIELD } else { DIALOG }))
            .border_1()
            .border_color(rgb(BORDER))
    }

    /// A title and description on the left, the control on the right.
    pub(super) fn setting_row(
        title: &'static str,
        description: &'static str,
        control: impl IntoElement,
    ) -> Div {
        div()
            .flex()
            .items_center()
            .gap_6()
            .py_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED))
                            .child(description),
                    ),
            )
            .child(control)
    }

    fn general_section(
        &self,
        content: Stateful<Div>,
        context: &mut Context<Self>,
    ) -> Stateful<Div> {
        let sunday_first = self.preferences.sunday_first;
        let tray_all_day = self.preferences.tray_all_day;
        let heading = |text: &'static str| {
            div()
                .text_size(px(20.0))
                .font_weight(FontWeight::SEMIBOLD)
                .mb_2()
                .child(text)
        };
        content
            .child(heading("Calendar view"))
            .child(Self::setting_row(
                "Start week on",
                "First day of the week in every view",
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Self::choice("week-start-monday", "Monday", !sunday_first).on_click(
                            context
                                .listener(|app, _, _, context| app.set_week_start(false, context)),
                        ),
                    )
                    .child(
                        Self::choice("week-start-sunday", "Sunday", sunday_first).on_click(
                            context
                                .listener(|app, _, _, context| app.set_week_start(true, context)),
                        ),
                    ),
            ))
            .child(
                div()
                    .py_3()
                    .border_t_1()
                    .border_color(rgb(BORDER))
                    .text_size(px(12.0))
                    .text_color(rgb(MUTED))
                    .child(format!(
                        "Display time zone: {} (system)",
                        self.timezone.name()
                    )),
            )
            .child(div().mt_4().child(heading("Tray list")))
            .child(Self::setting_row(
                "All-day events",
                "Show all-day events in the list that opens from the tray icon",
                div()
                    .flex()
                    .gap_1()
                    .child(
                        Self::choice("tray-all-day-on", "Show", tray_all_day)
                            .on_click(context.listener(|app, _, _, context| {
                                app.set_tray_all_day(true, context)
                            })),
                    )
                    .child(
                        Self::choice("tray-all-day-off", "Hide", !tray_all_day).on_click(
                            context.listener(|app, _, _, context| {
                                app.set_tray_all_day(false, context)
                            }),
                        ),
                    ),
            ))
    }

    pub(super) fn settings(&self, context: &mut Context<Self>) -> impl IntoElement {
        let content = div()
            .flex_1()
            .min_w_0()
            .p_6()
            .id("settings-content")
            .overflow_y_scroll();
        let content = match self.settings_section.as_str() {
            "accounts" => self.accounts_section(content, context),
            "working-hours" => self.working_hours_section(content, context),
            _ => self.general_section(content, context),
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
                    .bg(rgb(DIALOG))
                    .border_1()
                    .border_color(rgb(BORDER))
                    .shadow_lg()
                    .child(
                        div()
                            .w(px(235.0))
                            .flex_shrink_0()
                            .p_3()
                            .bg(rgb(NAVIGATION))
                            .child(
                                div()
                                    .p_2()
                                    .text_size(px(12.0))
                                    .text_color(rgb(MUTED))
                                    .child("Settings"),
                            )
                            .children(
                                [
                                    ("general", "General"),
                                    ("working-hours", "Working hours"),
                                    ("accounts", "Google accounts"),
                                ]
                                .into_iter()
                                .map(|(section, label)| {
                                    button(section, label)
                                        .w_full()
                                        .mb_1()
                                        .when(self.settings_section == section, |element| {
                                            element.bg(rgb(FIELD))
                                        })
                                        .on_click(context.listener(
                                            move |app, _, window, context| {
                                                app.select_settings_section(
                                                    section, window, context,
                                                );
                                            },
                                        ))
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
                                app.close_settings();
                                app.focus.focus(window, context);
                                context.notify();
                            })),
                    ),
            )
    }
}
