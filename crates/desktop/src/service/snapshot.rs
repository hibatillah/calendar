//! The cache as the views see it: accounts, calendars and the events of the covered range.

use super::*;
use calendar_sync::OperationKind;
use chrono_tz::Tz;
use std::collections::HashSet;

/// What the views need from the cache; rebuilt after every sync and every account change.
///
/// Event and calendar identifiers in here are view keys, `account/calendar[/event]`, so a calendar
/// shared between two accounts stays two separate calendars with separate toggles. Step 6 writes
/// must carry the account and calendar identifiers separately instead of splitting these.
#[derive(Clone, Default)]
pub struct Snapshot {
    /// False when this build has no Google OAuth client; nothing can be connected or refreshed.
    pub configured: bool,
    /// The zone the cached events were built in (the system zone at the last sync). `None` until
    /// the cache records one; views then keep the zone they have. Every day boundary uses it.
    pub zone: Option<Tz>,
    pub accounts: Vec<AccountView>,
    pub events: Vec<CalendarEvent>,
    /// The range the events above cover. Outside it the cache is silent, not empty.
    pub covered: Option<(DateTime<Utc>, DateTime<Utc>)>,
    /// Writes whose outcome is not known yet. Keep the optimistic placement of these events until
    /// they leave this list; `events` holds only what Google has confirmed.
    pub pending: Vec<PendingWrite>,
    /// View keys of events that are one occurrence of a recurring series.
    pub recurring: HashSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingWrite {
    pub operation_id: String,
    /// View key, like the event ids in `Snapshot::events`; for a create, the key the event will have.
    pub event_id: String,
    pub kind: OperationKind,
}

#[derive(Clone)]
pub struct AccountView {
    pub account_id: String,
    pub email: String,
    pub name: String,
    /// Calendar discovery, event reading and event writing are tracked apart. `None`: the
    /// credential is missing or unreadable and the account must reconnect.
    pub permissions: Option<Permissions>,
    /// The last refresh failed; the events below are still the saved ones.
    pub error: Option<String>,
    pub calendars: Vec<CalendarView>,
}

#[derive(Clone)]
pub struct CalendarView {
    pub key: String,
    pub summary: String,
    pub color: u32,
    /// The calendar role allows creating and changing events.
    pub writable: bool,
}

pub fn calendar_key(account_id: &str, calendar_id: &str) -> String {
    format!("{account_id}/{calendar_id}")
}

/// Inverse of [`calendar_key`]. Google account ids contain no `/`.
pub fn split_calendar_key(key: &str) -> Option<(&str, &str)> {
    key.split_once('/')
}

/// Splits an event id from `Snapshot::events` into `(account_id, calendar_id, event_id)`, the
/// separate identifiers a write needs. Cached event ids never contain `/`.
pub fn split_event_key(key: &str) -> Option<(&str, &str, &str)> {
    let (calendar_key, event_id) = key.rsplit_once('/')?;
    let (account_id, calendar_id) = split_calendar_key(calendar_key)?;

    Some((account_id, calendar_id, event_id))
}

pub(super) fn build_snapshot(
    store: &Store,
    vault: &Vault,
    errors: &HashMap<String, String>,
    configured: bool,
    now: DateTime<Utc>,
) -> Result<Snapshot, Error> {
    let mut snapshot = Snapshot {
        configured,
        zone: store.zone()?,
        ..Snapshot::default()
    };
    let mut covered: Option<(DateTime<Utc>, DateTime<Utc>)> = None;

    for account in store.accounts()? {
        let calendars = store.calendars(&account.account_id)?;
        for calendar in &calendars {
            if let Some(state) = store.sync_state(&account.account_id, &calendar.id)? {
                covered = Some(
                    covered.map_or((state.window_start, state.window_end), |(start, end)| {
                        (start.max(state.window_start), end.min(state.window_end))
                    }),
                );
            }
        }

        snapshot.accounts.push(AccountView {
            permissions: vault
                .load(&account.account_id)
                .ok()
                .flatten()
                .map(|secret| Permissions::from_scope(&secret.scope)),
            error: errors.get(&account.account_id).cloned(),
            calendars: calendars
                .iter()
                .map(|calendar| CalendarView {
                    key: calendar_key(&account.account_id, &calendar.id),
                    summary: calendar.summary.clone(),
                    color: u32::from_str_radix(
                        calendar.background_color.trim_start_matches('#'),
                        16,
                    )
                    .unwrap_or(0x6ba5d7),
                    writable: calendar.writable(),
                })
                .collect(),
            account_id: account.account_id,
            email: account.email,
            name: account.name,
        });
    }

    // Past events are pruned at sync time, so the past edge moves with the clock.
    snapshot.covered = covered.map(|(start, end)| (start.max(now - CACHE_PAST), end));
    if let Some((start, end)) = snapshot.covered {
        snapshot.events = store
            .events_between(start, end)?
            .into_iter()
            .map(|cached| {
                let key = calendar_key(&cached.account_id, &cached.google.event.calendar_id);
                let event_id = format!("{key}/{}", cached.google.event.event_id);
                if cached.google.recurring_event_id.is_some() {
                    snapshot.recurring.insert(event_id.clone());
                }
                CalendarEvent {
                    event_id,
                    calendar_id: key,
                    ..cached.google.event
                }
            })
            .collect();
    }

    snapshot.pending = store
        .pending_operations()?
        .into_iter()
        .map(|operation| PendingWrite {
            event_id: format!(
                "{}/{}",
                calendar_key(&operation.account_id, &operation.calendar_id),
                operation.event_id
            ),
            operation_id: operation.operation_id,
            kind: operation.kind,
        })
        .collect();

    Ok(snapshot)
}
