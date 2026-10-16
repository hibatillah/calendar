# Diagnostics and PostHog

Open **Activity & diagnostics** at the bottom of the desktop sidebar. It displays up to 500 recent rows, newest first, from bounded desktop/backend buffers. Desktop activity updates live; backend records refresh approximately every five seconds. The warning/error filter and Copy diagnostics work offline with the last backend snapshot. Rows are current-process diagnostics, not a permanent event-edit history. Daily JSON files remain in `.local/logs/` (seven per process).

## Enable uploads

No telemetry leaves the app by default. Put the project token in `.env` at the repository root (already ignored by Git). A blank template is provided in `.env.example`:

```dotenv
POSTHOG_ENABLED=true
POSTHOG_PROJECT_TOKEN=phc_REPLACE_WITH_PROJECT_TOKEN
POSTHOG_HOST=https://us.i.posthog.com
```

Use the project token from PostHog Project settings and the matching US host above or `https://eu.i.posthog.com`. Personal (`phx_`) and secret (`phs_`) keys are rejected. Restart both processes with `./scripts/start.ps1`. Set `POSTHOG_ENABLED=false` and restart to stop uploads. A token alone does not enable uploads.

Both apps parse `.env` without mutating process environment. Existing environment variables override `.env`. If none of these three settings exists, legacy `.local/telemetry.json` is still accepted with `enabled`, `project_token`, and `host`. Invalid configuration fails closed; the Activity status explains the category without exposing values. Run from the repository root. Only PostHog Cloud US/EU hosts are accepted.

The app and backend still run locally. Enabling this option sends the following telemetry to PostHog Cloud:

| Signal | Implementation |
| --- | --- |
| Analytics | `posthog-rs`: screen views, named button clicks, save requests/completions, move/resize gestures |
| Errors | SDK exceptions for handled worker/GraphQL failures and a sanitized panic hook; stack captured at the reporting site |
| Logs | Rust tracing actions, sanitized and exported through OpenTelemetry to `/i/v1/logs` |
| Traces | OpenTelemetry spans for desktop requests, Axum GraphQL execution and repository reads/writes, propagated through `traceparent`, exported to `/i/v1/traces` |

SDK events and OTLP requests use the same project token. Remote queues are bounded and delivery is best effort. Graceful shutdown allows up to three seconds of draining; forced termination can lose queued telemetry. Trace sampling currently captures all instrumented operations in this local preview. Introduce sampling and separate development/production configuration before a public release.

## Privacy and identity

Analytics properties contain static control names, component and app version, plus SDK runtime metadata. Each process persists an anonymous installation UUID in `.local/*-telemetry-id`; desktop and backend identities are intentionally separate until real authentication exists. No email, Google account ID, event ID, title, description, location or meeting URL is sent in analytics or remote log fields. Person-profile creation and GeoIP enrichment are disabled for analytics.

Remote logs use an explicit field allowlist. Raw error strings and panic payloads are replaced with stable error categories. Stack traces can contain source file paths and function names; error locations are reporting locations, not necessarily original failure sites. The dev profile strips debug info, so line-level resolution is limited. Release symbol uploads and public-product consent/preferences remain release work. Log files can include internal diagnostic errors; review before sharing them. The in-app copy button copies only sanitized activity records.

No native GPUI session replay or automatic browser click/page capture is implemented. Native screens are manual `screen_viewed` events. The Rust SDK also supports feature flags and experiments; those are available options, not enabled features of this app.

## Verify

1. Configure a real project token, enable uploads and restart.
2. Open Activity, navigate a week, open an event and save a sample change. Inspect `screen_viewed`, `control_clicked` and `event_save_completed` in PostHog events.
3. Check Logs for service `calendar-desktop` / `calendar-backend`; check Tracing for `backend_request` → `graphql_request` → `database.events` / `database.save`. Distributed tracing is currently a PostHog beta.
4. A genuine worker/GraphQL error should appear under Error Tracking. Tests cover error event generation without sending artificial errors to a live project.
5. Absence of a token means local-only behavior; “enabled” is configuration status, not proof of successful ingestion. Transport failures remain local diagnostics, never recursively exported.

References: [Rust SDK](https://posthog.com/docs/libraries/rust), [Rust errors](https://posthog.com/docs/error-tracking/installation/rust), [Rust logs](https://posthog.com/docs/logs/installation/rust), [OTLP traces](https://posthog.com/docs/distributed-tracing/start-here), [Rust debug symbols](https://posthog.com/docs/error-tracking/upload-source-maps/rust).
