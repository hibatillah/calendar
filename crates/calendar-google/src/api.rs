//! Calendar REST calls: calendar list, `syncToken` event reads, and conditional event writes.

use crate::{
    Error, Google, check,
    mapping::{
        Calendar, EventChange, GoogleEvent, RawEvent, draft_provider_id, event_body, map_event,
        provider_id,
    },
    parse,
};
use calendar_core::CalendarEvent;
use chrono::{DateTime, SecondsFormat, Utc};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use serde::{Deserialize, de::DeserializeOwned};

/// Partial responses keep pages small: attendees, conference data and the like are never downloaded.
const CALENDAR_FIELDS: &str =
    "nextPageToken,items(id,summary,primary,accessRole,backgroundColor,timeZone)";
const EVENT_FIELDS: &str = "nextPageToken,nextSyncToken,items(id,etag,status,summary,description,location,start,end,updated,eventType,recurringEventId,extendedProperties/private)";
const PAGE_SIZE: &str = "250";

pub enum EventQuery<'a> {
    /// First sync, or a resync after `Error::Gone`. The bounds apply to this request only: Google
    /// forbids `timeMin`/`timeMax` together with `syncToken`. Unbounded recurring series are
    /// expanded without limit when `time_max` is `None`, so callers should set both.
    Full {
        time_min: Option<DateTime<Utc>>,
        time_max: Option<DateTime<Utc>>,
    },
    /// Changes since the previous sync. Google forbids `timeMin`/`timeMax` here, so none is sent.
    Incremental { sync_token: &'a str },
}

pub struct EventSync {
    /// In the order Google returned them; apply them in order.
    pub changes: Vec<EventChange>,
    /// Store per calendar and send with `EventQuery::Incremental` next time.
    pub next_sync_token: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Page<T> {
    #[serde(default = "Vec::new")]
    items: Vec<T>,
    next_page_token: Option<String>,
    next_sync_token: Option<String>,
}

fn segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

fn bearer(token: &str) -> String {
    format!("Bearer {token}")
}

impl Google {
    pub fn list_calendars(&self, access_token: &str) -> Result<Vec<Calendar>, Error> {
        let mut calendars = Vec::new();
        self.each_item(
            access_token,
            &format!("{}/users/me/calendarList", self.endpoints.api),
            &[("maxResults", PAGE_SIZE), ("fields", CALENDAR_FIELDS)],
            |calendar: Calendar| {
                calendars.push(calendar);
                Ok(())
            },
        )?;

        Ok(calendars)
    }

    /// Reads every page or fails as a whole: a failed fetch is never returned as an empty result.
    pub fn sync_events(
        &self,
        access_token: &str,
        calendar: &Calendar,
        query: EventQuery,
    ) -> Result<EventSync, Error> {
        let bound = |value: DateTime<Utc>| value.to_rfc3339_opts(SecondsFormat::Secs, true);
        let (time_min, time_max);
        let mut params = vec![
            ("singleEvents", "true"),
            ("maxResults", PAGE_SIZE),
            ("fields", EVENT_FIELDS),
        ];
        match query {
            EventQuery::Full {
                time_min: lower,
                time_max: upper,
            } => {
                if let Some(lower) = lower {
                    time_min = bound(lower);
                    params.push(("timeMin", &time_min));
                }
                if let Some(upper) = upper {
                    time_max = bound(upper);
                    params.push(("timeMax", &time_max));
                }
            }
            EventQuery::Incremental { sync_token } => params.push(("syncToken", sync_token)),
        }

        let mut changes = Vec::new();
        let next_sync_token = self.each_item(
            access_token,
            &self.events_url(calendar, None),
            &params,
            |raw: RawEvent| {
                changes.push(map_event(raw, calendar, self.display_zone)?);
                Ok(())
            },
        )?;
        tracing::info!(
            action = "google_events_synced",
            change_count = changes.len()
        );

        Ok(EventSync {
            changes,
            next_sync_token: next_sync_token.ok_or(Error::InvalidResponse)?,
        })
    }

    /// Creates with a stable id derived from the draft's UUID. A retry of the same `operation_id`
    /// after an unknown outcome returns the event that the first attempt created.
    pub fn create_event(
        &self,
        access_token: &str,
        calendar: &Calendar,
        event: &CalendarEvent,
        operation_id: &str,
    ) -> Result<GoogleEvent, Error> {
        require_writable(calendar)?;
        let provider_id = draft_provider_id(&event.event_id)?;
        let body = event_body(event, self.display_zone, operation_id, Some(&provider_id));
        let result = self
            .agent
            .post(&self.events_url(calendar, None))
            .header("Authorization", bearer(access_token))
            .query("sendUpdates", "all")
            .send_json(&body);

        match parse::<RawEvent>(result) {
            Err(Error::Conflict) => self.reconcile(
                access_token,
                calendar,
                &provider_id,
                operation_id,
                Error::Conflict,
            ),
            other => self.mapped(other?, calendar),
        }
    }

    /// Patches with `If-Match: etag`. A stale ETag is `Error::PreconditionFailed`, unless the event
    /// already carries this `operation_id`, which means the same write was applied earlier.
    pub fn update_event(
        &self,
        access_token: &str,
        calendar: &Calendar,
        event: &CalendarEvent,
        etag: &str,
        operation_id: &str,
    ) -> Result<GoogleEvent, Error> {
        require_writable(calendar)?;
        let provider_id = provider_id(calendar, &event.event_id)?;
        let body = event_body(event, self.display_zone, operation_id, None);
        let result = self
            .agent
            .patch(&self.events_url(calendar, Some(provider_id)))
            .header("Authorization", bearer(access_token))
            .header("If-Match", etag)
            .query("sendUpdates", "all")
            .send_json(&body);

        match parse::<RawEvent>(result) {
            Err(Error::PreconditionFailed) => self.reconcile(
                access_token,
                calendar,
                provider_id,
                operation_id,
                Error::PreconditionFailed,
            ),
            other => self.mapped(other?, calendar),
        }
    }

    /// Deletes conditionally when `etag` is known. An already deleted event counts as done.
    pub fn delete_event(
        &self,
        access_token: &str,
        calendar: &Calendar,
        event_id: &str,
        etag: Option<&str>,
    ) -> Result<(), Error> {
        require_writable(calendar)?;
        let provider_id = provider_id(calendar, event_id)?;
        let mut request = self
            .agent
            .delete(&self.events_url(calendar, Some(provider_id)))
            .header("Authorization", bearer(access_token))
            .query("sendUpdates", "all");
        if let Some(etag) = etag {
            request = request.header("If-Match", etag);
        }

        match check(request.call()) {
            Ok(_) | Err(Error::Gone) => Ok(()),
            Err(error) => Err(error),
        }
    }

    /// Reads one event by the id the cache uses. `None` when Google no longer has it (404, 410 or a
    /// cancelled tombstone), which is a different answer from a failed read.
    pub fn get_event(
        &self,
        access_token: &str,
        calendar: &Calendar,
        event_id: &str,
    ) -> Result<Option<GoogleEvent>, Error> {
        self.fetch(access_token, calendar, provider_id(calendar, event_id)?)
    }

    fn fetch(
        &self,
        access_token: &str,
        calendar: &Calendar,
        provider_id: &str,
    ) -> Result<Option<GoogleEvent>, Error> {
        let raw = parse::<RawEvent>(
            self.agent
                .get(&self.events_url(calendar, Some(provider_id)))
                .header("Authorization", bearer(access_token))
                .call(),
        );

        match raw {
            Err(Error::NotFound | Error::Gone) => Ok(None),
            other => match map_event(other?, calendar, self.display_zone)? {
                EventChange::Upsert(event) => Ok(Some(*event)),
                EventChange::Deleted { .. } => Ok(None),
            },
        }
    }

    fn events_url(&self, calendar: &Calendar, provider_id: Option<&str>) -> String {
        let base = format!(
            "{}/calendars/{}/events",
            self.endpoints.api,
            segment(&calendar.id)
        );

        provider_id.map_or(base.clone(), |id| format!("{base}/{}", segment(id)))
    }

    fn mapped(&self, raw: RawEvent, calendar: &Calendar) -> Result<GoogleEvent, Error> {
        match map_event(raw, calendar, self.display_zone)? {
            EventChange::Upsert(event) => Ok(*event),
            EventChange::Deleted { .. } => Err(Error::InvalidResponse),
        }
    }

    /// After a 409/412: if the stored event carries our `operation_id`, the write already happened.
    fn reconcile(
        &self,
        access_token: &str,
        calendar: &Calendar,
        provider_id: &str,
        operation_id: &str,
        failure: Error,
    ) -> Result<GoogleEvent, Error> {
        let Some(current) = self.fetch(access_token, calendar, provider_id)? else {
            return Err(failure);
        };

        if current.operation_id.as_deref() == Some(operation_id) {
            tracing::info!(action = "google_write_already_applied");
            return Ok(current);
        }

        Err(failure)
    }

    /// Walks every page, handing items to `each`; returns the sync token from the last page.
    fn each_item<T: DeserializeOwned>(
        &self,
        access_token: &str,
        url: &str,
        params: &[(&str, &str)],
        mut each: impl FnMut(T) -> Result<(), Error>,
    ) -> Result<Option<String>, Error> {
        let mut page_token: Option<String> = None;

        loop {
            let mut query = params.to_vec();
            if let Some(token) = &page_token {
                query.push(("pageToken", token));
            }
            let page: Page<T> = parse(
                self.agent
                    .get(url)
                    .header("Authorization", bearer(access_token))
                    .query_pairs(query)
                    .call(),
            )?;
            for item in page.items {
                each(item)?;
            }

            match page.next_page_token {
                Some(next) if page_token.as_ref() != Some(&next) => page_token = Some(next),
                Some(_) => return Err(Error::InvalidResponse),
                None => return Ok(page.next_sync_token),
            }
        }
    }
}

fn require_writable(calendar: &Calendar) -> Result<(), Error> {
    if calendar.writable() {
        Ok(())
    } else {
        Err(Error::ReadOnly)
    }
}
