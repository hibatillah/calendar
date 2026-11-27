use super::*;
use crate::service::AccountView;

/// "Calendar list", "Event reading" and "Event editing" are separate grants and shown separately.
fn permission_line(account: &AccountView) -> String {
    let Some(permissions) = account.permissions else {
        return "Credentials unavailable. Reconnect this account.".into();
    };
    let state = |granted: bool| if granted { "allowed" } else { "not allowed" };

    format!(
        "Calendar list: {} · Event reading: {} · Event editing: {}",
        state(permissions.calendar_list),
        state(permissions.read_events),
        state(permissions.write_events)
    )
}

impl CalendarApp {
    pub(super) fn accounts_section(
        &self,
        content: Stateful<Div>,
        context: &mut Context<Self>,
    ) -> Stateful<Div> {
        let waiting = self.sign_in == SignInState::Waiting;
        let configured = self.snapshot.configured;

        content
            .child(div().text_size(px(22.0)).mb_4().child("Google accounts"))
            .child(
                div()
                    .text_size(px(12.0))
                    .mb_4()
                    .text_color(rgb(MUTED))
                    .child("Connect accounts and approve calendar access. Events are read from a local cache that Google keeps up to date."),
            )
            .children(self.snapshot.accounts.iter().enumerate().map(|(index, account)| {
                let disconnect_id = account.account_id.clone();
                div()
                    .mb_4()
                    .p_4()
                    .border_1()
                    .border_color(rgb(BORDER))
                    .rounded_lg()
                    .child(div().font_weight(FontWeight::SEMIBOLD).child(account.email.clone()))
                    .child(
                        div()
                            .mt_2()
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED))
                            .child(permission_line(account)),
                    )
                    .when_some(account.error.clone(), |element, error| {
                        element.child(
                            div()
                                .mt_2()
                                .text_size(px(12.0))
                                .text_color(rgb(0xe99a91))
                                .child(format!("Last refresh failed. {error}")),
                        )
                    })
                    .child(div().mt_3().children(account.calendars.iter().map(|calendar| {
                        div()
                            .py_1()
                            .text_size(px(12.0))
                            .child(calendar.summary.clone())
                    })))
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .mt_3()
                            .when(configured && !waiting, |element| {
                                element.child(
                                    div()
                                        .id(SharedString::from(format!("reconnect-{index}")))
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .bg(rgb(0x343434))
                                        .cursor_pointer()
                                        .child("Reconnect")
                                        .on_click(context.listener(|app, _, _, context| {
                                            app.handle.connect();
                                            context.notify();
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id(SharedString::from(format!("disconnect-{index}")))
                                    .px_3()
                                    .py_2()
                                    .rounded_md()
                                    .cursor_pointer()
                                    .text_color(rgb(0xe99a91))
                                    .child("Disconnect")
                                    .on_click(context.listener(move |app, _, _, context| {
                                        app.handle.disconnect(disconnect_id.clone());
                                        context.notify();
                                    })),
                            ),
                    )
            }))
            .when(configured && !waiting, |element| {
                element.child(button("add-google", "+ Add Google account").on_click(
                    context.listener(|app, _, _, context| {
                        app.handle.connect();
                        context.notify();
                    }),
                ))
            })
            .when(waiting, |element| {
                element
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED))
                            .child("Finish sign-in in your browser. Existing accounts stay connected."),
                    )
                    .child(button("cancel-sign-in", "Cancel sign-in").on_click(
                        context.listener(|app, _, _, context| {
                            app.handle.cancel_connect();
                            context.notify();
                        }),
                    ))
            })
            .when(!configured, |element| {
                element.child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(MUTED))
                        .child("Sign-in is not configured in this build."),
                )
            })
            .when_some(
                match &self.sign_in {
                    SignInState::Message(message) => Some(message.clone()),
                    _ => None,
                },
                |element, message| {
                    element.child(
                        div()
                            .mt_3()
                            .text_size(px(12.0))
                            .text_color(rgb(MUTED))
                            .child(message),
                    )
                },
            )
            .when_some(self.store_error.clone(), |element, error| {
                element.child(div().mt_3().text_color(rgb(0xe99a91)).child(error))
            })
    }
}
