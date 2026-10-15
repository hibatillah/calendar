use crate::google::{GoogleAuth, GoogleCalendar};
use async_graphql::SimpleObject;
use calendar_core::{CalendarEvent, EventCommand};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone, SimpleObject)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: String,
    pub picture: String,
    pub event_access: bool,
    pub write_access: bool,
    pub connecting: bool,
    pub message: String,
    pub calendars: Vec<GoogleCalendar>,
}
#[derive(Clone, SimpleObject)]
pub struct Connection {
    pub configured: bool,
    pub connected: bool,
    pub event_access: bool,
    pub account_key: String,
    pub connecting: bool,
    pub message: String,
    pub calendars: Vec<GoogleCalendar>,
    pub accounts: Vec<Account>,
}
#[derive(Clone)]
pub struct GoogleAccounts {
    sessions: Arc<Mutex<Vec<GoogleAuth>>>,
}
impl Default for GoogleAccounts {
    fn default() -> Self {
        let slots: Vec<String> = std::fs::read_to_string(".local/google-accounts.json")
            .ok()
            .and_then(|value| serde_json::from_str(&value).ok())
            .unwrap_or_else(|| vec!["desktop-account".into()]);
        Self {
            sessions: Arc::new(Mutex::new(
                slots
                    .into_iter()
                    .filter(|slot| slot == "desktop-account" || uuid::Uuid::parse_str(slot).is_ok())
                    .map(|slot| {
                        let mut session = GoogleAuth::default();
                        session.slot = slot;
                        session
                    })
                    .collect(),
            )),
        }
    }
}
fn persist(sessions: &[GoogleAuth]) -> Result<(), String> {
    std::fs::create_dir_all(".local").map_err(|_| "Cannot save account list.")?;
    let slots: Vec<_> = sessions.iter().map(|session| &session.slot).collect();
    std::fs::write(
        ".local/google-accounts.json",
        serde_json::to_vec(&slots).unwrap(),
    )
    .map_err(|_| "Cannot save account list.".into())
}
fn prefix_event(event: &mut CalendarEvent, slot: &str) {
    event.calendar_id = format!("{slot}|{}", event.calendar_id);
    event.event_id = format!("{slot}|{}", event.event_id);
}
impl GoogleAccounts {
    pub async fn status(&self) -> Connection {
        let sessions = self.sessions.lock().await.clone();
        let mut connection = Connection {
            configured: false,
            connected: false,
            event_access: false,
            account_key: String::new(),
            connecting: false,
            message: String::new(),
            calendars: vec![],
            accounts: vec![],
        };
        let mut duplicates = Vec::new();
        for session in sessions {
            let status = session.status().await;
            connection.configured |= status.configured;
            connection.connecting |= status.connecting;
            if !status.connected && !status.connecting {
                if !status.message.is_empty() {
                    connection.message = status.message;
                }
                continue;
            }
            connection.connected |= status.connected;
            connection.event_access |= status.event_access;
            if status.connected && !status.account_key.is_empty() {
                connection
                    .account_key
                    .push_str(&format!("{}:{};", session.slot, status.account_key));
            }
            let mut calendars = status.calendars;
            for calendar in &mut calendars {
                calendar.id = format!("{}|{}", session.slot, calendar.id);
            }
            if !status.email.is_empty()
                && let Some(previous) = connection
                    .accounts
                    .iter()
                    .position(|account| account.email == status.email)
            {
                let previous = connection.accounts.remove(previous);
                duplicates.push(previous.id.clone());
                connection
                    .calendars
                    .retain(|calendar| !calendar.id.starts_with(&format!("{}|", previous.id)));
            }
            connection.calendars.extend(calendars.clone());
            connection.accounts.push(Account {
                id: session.slot,
                name: status.name,
                email: status.email,
                picture: status.picture,
                event_access: status.event_access,
                write_access: status.write_access,
                connecting: status.connecting,
                message: status.message,
                calendars,
            });
        }
        if !duplicates.is_empty() {
            let mut sessions = self.sessions.lock().await;
            for session in sessions
                .iter()
                .filter(|session| duplicates.contains(&session.slot))
            {
                let _ = session.forget_local().await;
            }
            sessions.retain(|session| !duplicates.contains(&session.slot));
            let _ = persist(&sessions);
        }
        connection
    }
    pub async fn begin(&self, account_id: Option<String>) -> Result<String, String> {
        let mut sessions = self.sessions.lock().await;
        for session in sessions.iter() {
            if session.status().await.connecting {
                return Err("Finish or cancel the current Google sign-in first.".into());
            }
        }
        if let Some(account_id) = account_id {
            return sessions
                .iter()
                .find(|session| session.slot == account_id)
                .ok_or("Account not found.")?
                .begin()
                .await;
        }
        let mut session = GoogleAuth::default();
        session.slot = uuid::Uuid::new_v4().to_string();
        let url = session.begin().await?;
        sessions.push(session);
        persist(&sessions)?;
        Ok(url)
    }
    pub async fn disconnect(&self, account_id: Option<String>) -> Result<bool, String> {
        let mut sessions = self.sessions.lock().await;
        if let Some(identifier) = &account_id
            && let Some(session) = sessions.iter().find(|session| &session.slot == identifier)
        {
            let status = session.status().await;
            if status.connecting && status.connected {
                session.cancel().await;
                return Ok(true);
            }
        }
        for session in sessions.iter() {
            if account_id
                .as_ref()
                .is_none_or(|identifier| identifier == &session.slot)
            {
                session.disconnect().await?;
            }
        }
        sessions.retain(|session| {
            account_id
                .as_ref()
                .is_some_and(|identifier| identifier != &session.slot)
        });
        persist(&sessions)?;
        Ok(true)
    }
    pub async fn events(
        &self,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Result<Vec<CalendarEvent>, String> {
        let visible = self.status().await;
        let sessions = self.sessions.lock().await.clone();
        let mut events = vec![];
        for session in sessions {
            if !visible
                .accounts
                .iter()
                .any(|account| account.id == session.slot)
            {
                continue;
            }
            let status = session.status().await;
            if !status.connected || !status.event_access {
                continue;
            }
            if status.account_key.is_empty() {
                return Err(status.message);
            }
            let mut account_events = session.events(start, end).await?;
            for event in &mut account_events {
                prefix_event(event, &session.slot);
            }
            events.extend(account_events);
        }
        Ok(events)
    }
    pub async fn save(&self, mut command: EventCommand) -> Result<CalendarEvent, String> {
        let (slot, calendar) = command
            .event
            .calendar_id
            .split_once('|')
            .ok_or("Choose a connected Google calendar.")?;
        let slot = slot.to_string();
        let calendar = calendar.to_string();
        let sessions = self.sessions.lock().await;
        let session = sessions
            .iter()
            .find(|session| session.slot == slot)
            .cloned()
            .ok_or("Account disconnected.")?;
        drop(sessions);
        command.event.calendar_id = calendar;
        if command.expected_revision >= 0 {
            command.event.event_id = command
                .event
                .event_id
                .strip_prefix(&format!("{slot}|"))
                .ok_or("Event belongs to another account.")?
                .into();
        }
        let mut event = session.save(command).await?;
        prefix_event(&mut event, &slot);
        Ok(event)
    }
}
