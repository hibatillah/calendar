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

    pub(super) fn settings(&self, context: &mut Context<Self>) -> impl IntoElement {
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
