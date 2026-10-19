use super::*;

impl GoogleAuth {
    pub async fn events(
        &self,
        start: chrono::NaiveDate,
        end: chrono::NaiveDate,
    ) -> Result<Vec<calendar_core::CalendarEvent>, String> {
        if end <= start || (end - start).num_days() > 100 {
            return Err("Request a date range of 1 to 100 days.".into());
        }
        let connection = self.status().await;
        if !connection.connected {
            return Ok(Vec::new());
        }
        if !connection.event_access {
            return Err("Approve event access in the sidebar to load Google events.".into());
        }
        let mut state = self.state.lock().await;
        if let Some((cached_start, cached_end, fetched, events)) = &state.event_cache
            && *cached_start == start
            && *cached_end == end
            && fetched.elapsed() < Duration::from_secs(30)
        {
            return Ok(events.clone());
        }
        let credentials = state
            .credentials
            .as_ref()
            .ok_or("Google account disconnected.")?;
        let mut events = Vec::new();
        for calendar in &state.calendars {
            if calendar.access_role == "freeBusyReader" {
                continue;
            }
            let mut url =
                reqwest::Url::parse(&format!("{}/calendar/v3/calendars/", self.api_origin))
                    .unwrap();
            url.path_segments_mut()
                .unwrap()
                .pop_if_empty()
                .push(&calendar.id)
                .push("events");
            let mut page_token = String::new();
            loop {
                let mut request = self
                    .client
                    .get(url.clone())
                    .bearer_auth(&credentials.access_token)
                    .query(&[
                        ("singleEvents", "true".to_string()),
                        ("showDeleted", "false".to_string()),
                        ("maxResults", "2500".to_string()),
                        ("timeMin", format!("{start}T00:00:00+07:00")),
                        ("timeMax", format!("{end}T00:00:00+07:00")),
                    ]);
                if !page_token.is_empty() {
                    request = request.query(&[("pageToken", &page_token)]);
                }
                let response = request
                    .send()
                    .await
                    .map_err(|_| "Google Calendar is unreachable. Showing cached events.")?;
                if !response.status().is_success() {
                    let status = response.status().as_u16();
                    return Err(calendar_api_error(
                        status,
                        &response.json().await.unwrap_or_default(),
                    ));
                }
                let page: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|_| "Invalid Google event response.")?;
                for event in page["items"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                    if let Some(event) = map_google_event(event, calendar)? {
                        let mut event = event;
                        event.read_only |= !connection.write_access;
                        events.push(event);
                    }
                }
                match page["nextPageToken"].as_str() {
                    Some(next) if next != page_token => page_token = next.into(),
                    Some(_) => return Err("Google repeated an event pagination cursor.".into()),
                    None => break,
                }
            }
        }
        state.event_cache = Some((start, end, std::time::Instant::now(), events.clone()));
        tracing::info!(action = "google_events_synced", event_count = events.len());
        Ok(events)
    }

    pub async fn save(
        &self,
        command: calendar_core::EventCommand,
    ) -> Result<calendar_core::CalendarEvent, String> {
        command.event.validate()?;
        let connection = self.status().await;
        if !connection.write_access {
            return Err("Reconnect this account in Settings and approve calendar editing.".into());
        }
        let mut state = self.state.lock().await;
        let calendar = state
            .calendars
            .iter()
            .find(|calendar| calendar.id == command.event.calendar_id)
            .cloned()
            .ok_or("Calendar not found. Refresh and retry.")?;
        if !matches!(calendar.access_role.as_str(), "owner" | "writer") {
            return Err("This calendar is read-only.".into());
        }
        let credentials = state.credentials.as_ref().ok_or("Account disconnected.")?;
        let event = &command.event;
        let mut collection =
            reqwest::Url::parse(&format!("{}/calendar/v3/calendars/", self.api_origin)).unwrap();
        collection
            .path_segments_mut()
            .unwrap()
            .pop_if_empty()
            .push(&calendar.id)
            .push("events");
        let creating = command.expected_revision < 0;
        let provider_id = if creating {
            uuid::Uuid::parse_str(&event.event_id)
                .map_err(|_| "Invalid new event identifier.")?
                .simple()
                .to_string()
        } else {
            event
                .event_id
                .rsplit(':')
                .next()
                .ok_or("Invalid event identifier.")?
                .to_string()
        };
        let mut item = collection.clone();
        item.path_segments_mut().unwrap().push(&provider_id);
        let date_value = |instant: chrono::DateTime<chrono::Utc>| {
            if event.all_day {
                serde_json::json!({"date":instant.with_timezone(&chrono_tz::Asia::Jakarta).date_naive().to_string()})
            } else {
                serde_json::json!({"dateTime":instant.to_rfc3339(),"timeZone":event.timezone})
            }
        };
        let mut payload = serde_json::json!({"summary":event.title,"description":event.description,"location":event.location,"start":date_value(event.start),"end":date_value(event.end)});
        let request = if creating {
            payload["id"] = provider_id.clone().into();
            payload["extendedProperties"] =
                serde_json::json!({"private":{"calendarOperation":command.operation_id}});
            self.client.post(collection)
        } else {
            let current = self
                .client
                .get(item.clone())
                .bearer_auth(&credentials.access_token)
                .send()
                .await
                .map_err(|_| "Cannot check the current Google event.")?;
            if !current.status().is_success() {
                return Err("Event no longer available. Refresh before editing.".into());
            }
            let current: serde_json::Value =
                current.json().await.map_err(|_| "Invalid Google event.")?;
            if provider_operation(&current) == Some(command.operation_id.as_str()) {
                return map_google_event(&current, &calendar)?.ok_or("Event was removed.".into());
            }
            let mapped = map_google_event(&current, &calendar)?.ok_or("Event was removed.")?;
            if mapped.read_only {
                return Err("This Google event type is read-only.".into());
            }
            if mapped.revision != command.expected_revision {
                return Err("Event changed in Google. Refresh before saving again.".into());
            }
            let mut private = current["extendedProperties"]["private"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            private.insert(
                "calendarOperation".into(),
                command.operation_id.clone().into(),
            );
            payload["extendedProperties"] = serde_json::json!({"private":private});
            self.client.patch(item.clone()).header(
                "If-Match",
                current["etag"].as_str().ok_or("Missing event version.")?,
            )
        };
        let response = request
            .bearer_auth(&credentials.access_token)
            .query(&[("sendUpdates", "all")])
            .json(&payload)
            .send()
            .await
            .map_err(|_| "Google save could not be confirmed. Refresh before retrying.")?;
        let status = response.status().as_u16();
        let result: serde_json::Value = if creating && status == 409 {
            let response = self
                .client
                .get(item)
                .bearer_auth(&credentials.access_token)
                .send()
                .await
                .map_err(|_| "Cannot reconcile event creation.")?;
            if !response.status().is_success() {
                return Err("Cannot reconcile event creation.".into());
            }
            let value: serde_json::Value = response
                .json()
                .await
                .map_err(|_| "Invalid Google event response.")?;
            if provider_operation(&value) != Some(command.operation_id.as_str()) {
                return Err("Event identifier already exists.".into());
            }
            value
        } else {
            if status == 412 {
                return Err("Event changed in Google. Refresh before saving again.".into());
            }
            if !(200..300).contains(&status) {
                return Err(calendar_api_error(
                    status,
                    &response.json().await.unwrap_or_default(),
                ));
            }
            response.json().await.map_err(|_| "Google saved the event but returned an unreadable response. Refresh before retrying.")?
        };
        state.event_cache = None;
        tracing::info!(action = "google_event_saved", creating);
        map_google_event(&result, &calendar)?.ok_or("Event not available after saving.".into())
    }
}
