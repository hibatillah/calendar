# Architecture

Calendar is a single Windows desktop application: a GPUI window plus a tray icon, talking to the Google Calendar REST API directly. There is no backend service, no web client and no mobile client.

## Status

The repository has been reduced to the Windows desktop app. The desktop app, however, still reads its data through a local GraphQL backend that has been removed from the tree (last present at commit `2a45ab3`). Until the migration below is finished, `apps/desktop/src/service.rs` has no data source and the app will not show Google data.

Porting reference (all at `2a45ab3`): `apps/backend/src/google/authorization.rs` (PKCE, loopback callback), `credentials.rs` (Credential Manager), `events.rs` (reads, creates, updates), `mapping.rs` (Google ↔ `CalendarEvent`), `google/tests.rs` (fake-endpoint tests worth keeping).

## Target runtime shape

```text
GPUI window  ◄── snapshots ──┐        tray icon + menu
     │ commands              │             │ open / quit / agenda
     ▼                       │             ▼
 service.rs (channel bridge) ┴──► sync engine (one Tokio current-thread runtime, off the UI thread)
                                      │            │
                          calendar-google      store (SQLite cache, sync tokens, journal)
                          REST + OAuth         Credential Manager (refresh tokens)
```

- OAuth 2.0 authorization-code flow with PKCE and a loopback redirect on `127.0.0.1`. Refresh tokens live in Windows Credential Manager, access tokens in memory only.
- Reads use `events.list` with `syncToken` after the first full sync. A `410 Gone` clears that calendar's token and triggers a full resync. Writes use stable event IDs and ETags (`If-Match`).
- `events.watch` is not used: it needs a public HTTPS endpoint. Sync is polled.
- The UI never performs network or database I/O. It shows cached data first and re-renders when a sync result arrives.

### Sync triggers

| State                         | Behavior                                              |
| ----------------------------- | ----------------------------------------------------- |
| Window open                   | Poll every 30–60 s                                    |
| Window closed, running in tray | Poll every 2–5 min                                    |
| Window opened, window focused, tray menu opened | One immediate sync                          |
| Sync finished a few seconds ago | Skip the trigger                                    |
| Sync already running          | Join it; never start a second one                     |

Intervals are initial values to tune against measured request volume and battery impact.

### Closing to the tray

Closing the window should drop the GPUI window so its renderer resources are released, leaving the process with the tray, sync timer and local reminders. Whether GPUI returns enough memory when the last window closes is **unmeasured**. If the in-tray footprint stays too high, the fallback is a split: a tiny tray/sync agent executable that launches the UI executable on demand. Do not build the split before measuring.

## Proposed repository layout

```text
calendar/
├── Cargo.toml                  workspace; Cargo.lock
├── apps/
│   └── desktop/                the only binary (calendar-desktop.exe)
│       └── src/
│           ├── main.rs         startup, single instance, window lifecycle
│           ├── app.rs, app/    GPUI state, views, gestures, editor (existing)
│           ├── platform.rs     tray, window placement, Windows-only APIs
│           ├── notifications.rs  local reminders from cached events (new)
│           └── service.rs      channel bridge between UI and sync engine
├── crates/
│   ├── calendar-core/          domain types and time rules (existing; no GPUI/HTTP/storage)
│   ├── calendar-layout/        overlap and column layout (existing; pure)
│   ├── calendar-google/        new: Google REST client, OAuth/PKCE, token refresh,
│   │                           Google ↔ core mapping. No GPUI, no storage.
│   ├── calendar-sync/          new: SQLite cache, sync tokens, operation journal,
│   │                           credential vault, sync scheduler/coalescing
│   └── telemetry/              diagnostics (existing; see open questions)
├── assets/fonts/               bundled Inter + licence
├── installer/windows/          WiX MSI authoring
├── scripts/                    Bash entry points + scripts/windows/ PowerShell helpers
├── docs/                       architecture, google setup, telemetry
├── .github/workflows/          ci.yml, release.yml
├── .claude/, .codex/           rules and review agents
└── AGENTS.md
```

Why two new crates rather than modules inside `apps/desktop`: the Google client and the sync/storage layer can be tested on any OS with a fake HTTP server and an in-memory SQLite database, which keeps CI feedback off the Windows runner and keeps the existing rule that domain and layout crates do not depend on GPUI, HTTP or storage. If that testing benefit turns out not to matter, merging `calendar-google` and `calendar-sync` into one crate is a reasonable simplification.

## Migration steps remaining

1. Create `calendar-google` from the porting reference; replace GraphQL-embedded types with plain structs and drop `async-graphql`/`axum` except a minimal loopback listener for the OAuth callback.
2. Create `calendar-sync`: move the desktop cache and journal out of `service.rs`, add `sync_token` storage per calendar, implement the trigger/coalescing table above.
3. Rewire `service.rs` to the sync engine; remove the GraphQL query strings and loopback transport.
4. Add tray-resident mode (drop window on close), then local reminders.
5. Measure idle working set in both window-open and in-tray states and record the numbers here.

## Time and identifiers

Calendar types use Chrono + chrono-tz. Named-zone conversion is explicit; nonexistent and ambiguous local times produce errors. Events store UTC instants and a named zone. All-day values use midnight boundaries in the display zone with an all-day flag, preserving Google's exclusive end dates. Account-prefixed identifiers keep shared calendars from different accounts separate.

## UI module map

`app.rs` owns state and lifecycle; `app/shell.rs` composes the window. `app/navigation.rs`, `gestures.rs`, `editor.rs`, `timeline.rs`, `sidebar.rs`, `settings.rs` and `agenda.rs` own their interactions and rendering. Shared element builders live in `controls.rs`; headless interaction tests are in `app/tests.rs`.
