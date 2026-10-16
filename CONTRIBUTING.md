# Contributing

Keep identifiers explicit: `event_id`, `calendar_id`, `operation_id`. Use Rust formatting and the checks in the README. Keep changes local unless publishing is explicitly requested.

After the initial repository bootstrap, make every change on a feature branch and open a pull request into `main`. Let CI finish and resolve failures before merging. Do not push changes directly to `main`. Creating a PR does not authorize merging it.

## Where changes belong

- `crates/calendar-core`: domain types, validation and time rules. No GUI, HTTP or database dependency.
- `crates/calendar-layout`: pure event placement and overlapping-column calculation.
- `apps/desktop/src/app.rs`: presentation, editor and gestures.
- `apps/desktop/src/platform.rs`: native backdrop/tray placement. Keep Windows imports behind target gates.
- `apps/desktop/src/service.rs`: background GraphQL calls, cache and operation journal.
- `apps/backend/src/repository.rs`: storage transactions and conflict/idempotency rules.
- `crates/telemetry`: local diagnostics, PostHog events/errors and OpenTelemetry log/trace export.

## Testing

Use ordinary Rust tests for calendar mathematics, `#[tokio::test]` plus Tower `ServiceExt::oneshot` for real Axum route behavior, and `#[gpui::test]` with `TestAppContext` for GUI state transitions. Tests must use isolated memory databases or test-owned files, never the running preview database.

Add cases for behavior that can lose or misplace time: overlap clusters, midnight, leap dates, DST gaps/repeated hours, stale writes and repeated operation IDs. Test requested behavior rather than implementation details.

Before declaring native UI work verified, check opaque header/sidebar colors and the Activity panel, minimize/maximize/restore, resized layouts, drag/resize cancellation, offline event viewing, and tray activation/quit. Compilation and headless tests do not prove native appearance.

macOS builds need a Mac with the Xcode tools. Use normal `cargo` commands there; do not introduce Windows dependencies into shared crates. Keep traffic-light space, menu-bar template icons and native window lifecycle in mind. No macOS support claim until it has been built and exercised there.

## Logs and privacy

Log meaningful state changes with `action`, object/operation IDs and outcome. Never log OAuth credentials, session tokens, full API requests, or event content. Use `RUST_LOG` for filtering and `CALENDAR_LOG_DIR` to select a local directory. Cache/journal files contain event content; logs are diagnostics, not an audit backup.

References: [Axum's testing example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs), [GPUI test context](https://docs.rs/gpui/latest/gpui/attr.test.html).


For telemetry, use static screen/control/error categories. Instrument async operations with `#[tracing::instrument(skip_all)]` so event bodies and tokens never become span attributes. Remote logs use an allowlist; do not expand it to free-form user data. Keep transport logs out of remote export to prevent feedback loops. See [telemetry](docs/telemetry.md) for configuration and verification.

Document GraphQL fields and arguments alongside their Rust definitions. Keep examples in `docs/graphql.md` aligned with the exported SDL. Tests should cover authenticated introspection, query/mutation behavior and error extension codes.
