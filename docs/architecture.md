# Architecture and boundaries

## TypeScript workspace decision

The TypeScript clients use **Bun workspaces** with one committed `bun.lock`. Use `workspace:*` for shared API, calendar helpers and design-token packages. Pin Bun in the root package manifest and CI; use frozen-lockfile installs. Keep Node available for Expo and tooling that requires it; Bun as package manager does not replace Metro, native build tools or the mobile JavaScript engine.

Use **Turborepo** with those applications to coordinate shared-package dependencies, type checking, tests and web builds. Cache deterministic outputs only; disable caching for development servers, OAuth flows, deployment and signing. Keep native Android/iOS builds in their dedicated CI jobs. Cargo remains the Rust workspace/build system, with its own lockfile and cache. Do not wrap every Rust crate in a JavaScript package or duplicate Cargo's build graph.

The web app and shared TypeScript packages implement this workspace. The mobile app builds on the same API and calendar packages. References: [Bun workspaces](https://bun.sh/docs/pm/workspaces), [Turborepo installation](https://turborepo.com/docs/getting-started/installation), [Expo with Bun](https://docs.expo.dev/guides/using-bun/).

## Current local application

```text
GPUI window / tray agenda
        │ commands and snapshots (channels)
        ▼
Background service ── SQLite desktop cache + operation journal
        │ GraphQL over loopback, local token
        ▼
Axum / async-graphql ── repository ── SQLite backend store
```

The UI never performs network or database I/O on its rendering thread. Both windows read the same cache and backend. The backend commits an event and its idempotency result in one transaction. Retries with the same ID and same request return the committed result; different data under that ID is rejected. Expected revisions prevent stale writes.

The first local implementation uses SQLite because it runs without a database service. PostgreSQL remains the hosted-product target; this is a deliberate local-preview deviation from the initial plan. SQL stays in the backend repository. Schema initialization is currently small and inline; versioned migrations are required before changing an existing release schema.

The desktop stores pending commands before sending them. Transport failures are marked `needs_reconciliation`, not silently replayed. Automatic recovery/replay is not implemented yet. A server read refresh restores authoritative events; a durable operation-result endpoint is the next step before enabling offline edits.

Rust doc comments on schema types, fields and resolver arguments become GraphQL descriptions. Authenticated introspection powers the local GraphiQL docs explorer. The backend exports SDL to `contracts/graphql/schema.graphql` and serves it at `/schema.graphql`. The desktop uses explicit GraphQL operations and Serde. Introduce generated Rust and TypeScript clients as the schema grows.

## Platform readiness

Calendar types, storage and layout are shared Rust. Native backdrop/tray integration is in `platform.rs`; Windows work-area and DPI APIs are target-gated. GPUI handles the native renderer. macOS gets transparent title-bar options, traffic-light space, a template tray icon and a top-screen popup fallback. macOS packaging, lifecycle differences and actual rendering still require a Mac.

Desktop currently uses Chrono + chrono-tz rather than the initial Jiff candidate. Named-zone conversion is explicit; nonexistent and ambiguous local times produce errors. Events store UTC instants and a named zone. All-day values use midnight boundaries in the display zone and an all-day flag, preserving provider exclusive end dates.

## Google integration boundary

Implemented: desktop OAuth with PKCE/state, per-account OS credential storage, calendar discovery, paginated event reads with recurring-instance expansion, and account/month-specific offline caches. The desktop refreshes a 56-day range every 30 seconds while idle and requests a new range when the displayed month changes. Calendar roles and OAuth permissions control editability. Creates use stable IDs and updates use ETags; operation markers reconcile retries. Account-prefixed identifiers keep shared calendars separate. Legacy preview rows and caches are removed on startup.

Still required: incremental sync and invalidation recovery, series-wide recurring edits, attendee management, configurable display zones and hosted multi-user isolation. All-day provider dates map to midnight in the fixed Asia/Jakarta display zone with exclusive end dates preserved. Public multi-user web/mobile hosting remains deferred; the web development bridge reuses this local backend.

## UI module map

Desktop `app.rs` owns state and lifecycle; `app/shell.rs` composes the window. `app/navigation.rs`, `gestures.rs`, `editor.rs`, `timeline.rs`, `sidebar.rs`, `settings.rs` and `agenda.rs` own their corresponding interactions and rendering. Shared element builders live in `controls.rs`; headless interaction tests are in `app/tests.rs`.

Web `use-calendar-controller.ts` owns query state and actions. `calendar-app.tsx` composes `calendar-sidebar`, `mini-calendar`, `week-calendar`, `month-calendar`, `calendar-settings` and `event-editor`. Browser snapshot storage has a separate module. Marketing illustrations and their styles live in `product-previews.tsx` and `product-previews.css`, separate from page content.

Mobile `use-mobile-calendar.ts` owns connection, query, preference and reminder state. `calendar-screen.tsx` composes the month, agenda and sheets; `mobile-month`, `mobile-settings` and `mobile-editor` own their views. `components.tsx` contains shared native controls. Platform-specific persistence, native date fields and notification adapters retain their existing `.native.ts` resolution.

Backend `google.rs` owns shared provider state and discovery; `google/authorization.rs`, `credentials.rs`, `events.rs` and `mapping.rs` separate authorization, OS vault access, provider writes/reads and normalization. Provider tests remain isolated with fake HTTP endpoints. Telemetry configuration and telemetry tests are separated from logging/export initialization.

The refactor preserves API schema, event identities, OAuth scopes, storage keys, migration behavior, optimistic write/rollback behavior and notification scheduling. Small domain and storage modules remain cohesive rather than being split just to reduce line counts.
