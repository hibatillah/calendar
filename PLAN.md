# Calendar: Windows desktop implementation plan

Status: local desktop/backend preview implemented and running. Connected Google release remains outstanding; see the implementation status below.

## Implementation status (14 October 2026)

- Implemented: Cargo workspace; local Axum/GraphQL backend; SQLite event store and desktop cache; GPUI month/week UI, editor, move/resize gestures, all-day row and tray agenda; local structured rotating logs; domain, Axum and headless GPUI tests.
- Visual revisions: neutral black/gray surfaces, 88% root opacity, compact 216px sidebar and 122px combined header rows.
- macOS-ready boundaries from the beginning: Windows native APIs are target-gated; shared UI/domain/service logic is portable; traffic-light spacing and tray template icons are accounted for. A macOS build has not been verified.
- Local-development decisions: SQLite backend until hosted infrastructure is needed; Chrono + chrono-tz; explicit GraphQL operations before generated clients.
- Google account milestone: system-browser Desktop OAuth with PKCE/state, temporary loopback callback, OS credential-vault persistence, token refresh, calendar-list discovery and disconnect implemented. Live consent awaits Google Cloud credentials; see docs/google.md.
- Still outstanding: Google event sync and mutations, actual multi-user account isolation, PostgreSQL migration path, automatic ambiguous-operation reconciliation, generated GraphQL clients, and public distribution.
- Native visual/interaction verification was blocked because Computer Use did not approve access to the new application. User feedback confirmed the app was visible and informed styling changes. Headless UI tests do not replace that native verification.

## Product decisions

- Personal use first, with account isolation and a straightforward path to public onboarding.
- Windows desktop first, using Rust and GPUI. Backend in Rust with Axum and GraphQL.
- Google Calendar is the only calendar provider and source of truth for events.
- React Native mobile and TanStack Start web are later phases; do not scaffold them yet.
- View cached events offline. Keep the mutation architecture suitable for future offline editing, but disable new edits while offline in this release.
- Desktop scope: view, create, edit, drag events between dates/times, and resize start/end times.
- Full-height sidebar with a complete month grid (28–31 dates plus adjacent-month cells), account/calendar list, and selected-week highlight. Main area displays seven days.
- Custom window chrome: sidebar continues to the top edge, no separate conventional title strip. Sidebar and top toolbar are translucent; text and controls remain opaque.
- System tray icon opens a compact agenda without opening the main calendar.

## First review goal

Deliver a runnable Windows executable and local backend, with a clearly labeled sample-data mode so visual review does not depend on Google credentials. This is the first checkpoint, not completion of the connected calendar release.

The first preview includes the translucent shell, month sidebar, week grid, event details, local sample-event creation/editing/dragging/resizing, and tray agenda. It must launch on this machine and be visually checked. Provide exact run commands and explain which features use samples.

## Workspace and ownership

Use a Cargo workspace. Introduce a TypeScript workspace when mobile or web implementation starts.

```text
apps/
  desktop/                 GPUI executable, UI state, tray, SQLite cache
  backend/                 Axum executable, auth, GraphQL, database, sync jobs
    migrations/            PostgreSQL schema migrations
crates/
  calendar-core/           Provider-independent event/time types and commands
  calendar-layout/         Pure week-grid geometry and overlap calculations
  api-client/              Typed GraphQL operations and transport
  google-calendar/         Google transport, mapping, pagination and errors
contracts/
  graphql/                 Exported documented GraphQL schema
fixtures/
  calendar/                Shared calendar correctness examples
```

Create crates as their implementation begins. Keep backend auth and database access as modules initially. Keep desktop SQLite storage separate from backend PostgreSQL storage. Domain code must not depend on GPUI, SQLx, or HTTP framework types.

## Technical choices

- GPUI + GPUI Kit: custom calendar rendering and standard controls. Pin a compatible dependency set after the Windows smoke test.
- Axum + Tokio + async-graphql + async-graphql-axum: one backend process with background sync tasks initially.
- PostgreSQL + SQLx: explicit queries and migrations. SQLite through SQLx for desktop cached data.
- reqwest + Serde: explicit desktop GraphQL requests. Export SDL for eventual TypeScript code generation; serve GraphiQL locally.
- Jiff: preferred candidate for date/time and named-zone calculations; validate serialization and database adapters before locking the choice.
- tray-icon: preferred tray integration candidate. Prove Windows message-loop integration with GPUI before building the full popup.
- OS credential storage for desktop sessions; encrypted provider refresh tokens on the backend with encryption keys outside the database.
- tracing for diagnostics, with tokens and private event descriptions excluded from logs.

## Milestone 1: Windows shell and tray feasibility

1. Create the Cargo workspace, desktop executable, minimal backend health endpoint and GraphQL schema, and documented local run commands.
2. Verify MSVC and Windows SDK tooling; ensure Microsoft's linker is selected instead of Git's link.exe.
3. Build a custom chrome layout: full-height left sidebar, toolbar above the main grid, and minimize/maximize/close controls at top right.
4. Configure the supported Windows translucent/blurred backdrop. Keep the GPUI component root and title-bar layers transparent; apply tint to intended surfaces only. Do not reduce opacity of the entire window.
5. Preserve resize borders, window dragging from empty chrome, double-click maximize, and clickable toolbar controls without accidental window dragging.
6. Create a tray icon and a small GPUI agenda popup. Route tray events into GPUI safely through the Windows message loop.
7. Position the popup near the tray activation location, clamp it to the monitor work area, account for DPI scaling, and dismiss it on outside focus loss or Escape.
8. Default close behavior: hide to tray while the tray is active. Right-click menu offers Open Calendar and Quit. If tray initialization fails, closing must not leave an inaccessible background process.

Acceptance: executable launches; header and sidebar translucency are visually verified over light and dark backgrounds; minimize/maximize/restore work; tray popup works with the main window hidden; Quit exits cleanly. Record OS/backdrop limitations and provide a readable fallback when system transparency is disabled.

## Milestone 2: Interactive desktop preview

1. Add calendar domain fixtures and a clearly labeled Preview mode, separate from connected-account storage.
2. Render a month navigator with selected day/week, month navigation, and Today action. Default to Monday-first, local display time zone, and a seven-day main view.
3. Render the hourly grid, all-day row, date headers, current-time marker, calendar colors, and calendar visibility toggles.
4. Position events by start/end time. Lay overlapping events side by side; split multi-day visual segments while preserving one event identity.
5. Add event-detail popover and create/edit form. Support title, calendar, date/time, all-day flag, description, location, and display of conferencing links.
6. Click-drag on empty grid to create; drag event bodies to move; drag top/bottom edges to change start/end. Use 15-minute snapping initially, preserve duration on move, preview the new range, and support Escape cancellation.
7. Add edge auto-scroll during gestures and reject non-positive durations. Changes to time should use the displayed zone, with explicit handling of daylight-saving gaps and repeated hours.
8. Tray popup shows ongoing and upcoming events for today, plus the next upcoming event when today is empty. Selecting an event opens its details in the main window. Show empty and loading states.
9. Keep layout calculations separate from GPUI rendering. Keep network/database work off the UI thread.

Acceptance: user can launch the preview and review the requested interactions before Google setup. Test overlap clusters, adjacent events, midnight boundaries, all-day events, daylight-saving cases, and drag/resize conversion. Show that sample mode is not connected to Google.

## Milestone 3: Google account connection and cached viewing

1. Add PostgreSQL migrations for Calendar users, sessions, connected Google accounts, calendars, provider events, sync cursors, and sync-job state. Every account/calendar/event query is scoped to the authenticated user.
2. Separate Calendar identity from connected Google accounts; allow the model to hold multiple Google accounts per user.
3. Implement system-browser authorization with state, PKCE where applicable, and a short-lived one-time desktop handoff. Never embed a confidential client secret in the desktop app or pass long-lived session tokens in redirect URLs.
4. Validate the desktop sign-in design against Google's client-type and redirect requirements before implementation. Document Google Cloud project, consent screen, credentials, and Calendar API setup.
5. Implement calendar discovery, access roles, initial paginated event sync, incremental sync, cancellation handling, and invalid-sync-token recovery. Preserve recurrence masters, exceptions, original occurrence identity, and provider ETags.
6. Treat a requested week range and a provider incremental-sync cursor as different concerns. Use parameters compatible with Google's sync rules; resolve recurring occurrences for bounded visible ranges and store coverage explicitly.
7. Add background refresh, backoff, and reconnect states. Public HTTPS webhook ingestion and channel renewal can follow the local polling implementation; webhooks signal a need to sync rather than supplying event data.
8. Implement SQLite cache updates transactionally and render from cache immediately. Store account scope, covered ranges, and last successful sync time. Clear relevant local data on disconnect/sign-out.
9. Display offline/stale state and distinguish an uncached week from a genuinely empty week. The tray uses the same cache and freshness indicators.

Acceptance: connect a real Google account, view its calendars/events, restart offline and view previously cached ranges, reconnect and receive changes without duplicates. Real-data validation requires user-provided OAuth configuration; do not present fixtures as successful Google integration.

## Milestone 4: Online event editing with offline-ready foundations

1. Route all edits through domain commands: CreateEvent, UpdateEvent, MoveEvent, and ResizeEvent. UI components never call Google directly.
2. Give each command an operation ID, target account/calendar/event identity, expected provider version, payload, and status. Persist a local operation journal for in-flight recovery and backend deduplication.
3. In this release, reject new offline edits before changing durable event state. Preserve ambiguous in-flight operations for reconciliation rather than blindly replaying them.
4. Apply optimistic online previews with pending state. Reconcile against Google's confirmed response, and show or roll back failures without discarding the user's proposed values.
5. Use conditional provider writes where supported to detect external changes. On conflict, refetch and show the current event with a clear retry/edit path. Handle rate limits, expired authorization, and read-only calendars.
6. Make create retries duplicate-safe using a stable provider-compatible event ID or an equivalent reconciliation mechanism. Do not claim exactly-once behavior solely from a backend operation ID.
7. Support ordinary events and recurring-event edits with an explicit This occurrence / Entire series choice. Treat This and following as a separate implementation because it requires splitting a series. Hide unsupported choices rather than silently applying the wrong scope.
8. Preserve fields not edited by the client. Respect organizer/calendar permissions, show attendee effects before saving, and make notification behavior explicit.
9. Keep tray and main-window state consistent after successful mutations and external refreshes.

Acceptance: live create/edit/move/resize succeeds on a designated test calendar; retry does not duplicate events; stale-version conflicts are visible; recurring occurrences keep correct identity; offline editing is visibly unavailable. Tests use fixtures or a test calendar, never unrelated personal events.

## Validation and completion gates

- Run cargo fmt, workspace checks/build, and focused tests for calendar layout, time boundaries, sync pagination/recovery, ownership isolation, and mutation retry/conflict behavior.
- Add contract checks so the exported schema and generated client cannot silently drift.
- Perform actual Windows interaction checks: title-bar drag and buttons, translucency, resized/maximized layouts, event gestures, tray open/close, and offline restart.
- First review is complete when the preview executable and local backend run and the user receives launch instructions plus verified/unverified status.
- Connected desktop scope is complete after Google viewing and online mutation acceptance checks. A compiling preview alone is not completion of Google integration.

## Later work

- Enable offline command creation, conflict-resolution UI, and queued replay using the established operation journal.
- Public release: Google consent/verification requirements for chosen scopes, hosted backend, privacy/deletion flows, signed Windows distribution/update mechanism, and unattended sync reliability.
- macOS support, mobile, and web after Windows review and connected desktop completion.

## Sources checked during planning

- GPUI Kit title bar: https://gpui-kit.com/component/title-bar/
- Window backdrop options: https://docs.rs/gpui-kit/latest/gpui_kit/enum.WindowBackgroundAppearance.html
- Tray event-loop requirements: https://docs.rs/tray-icon/latest/tray_icon/index.html
- Google incremental sync: https://developers.google.com/workspace/calendar/api/guides/sync
- Google push notifications: https://developers.google.com/workspace/calendar/api/guides/push
- Google conditional writes: https://developers.google.com/calendar/api/guides/version-resources
- Recurring events: https://developers.google.com/workspace/calendar/api/guides/recurringevents
