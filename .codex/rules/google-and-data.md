# Google, persistence and telemetry

- Read `docs/google.md` before changing OAuth or provider calls. Keep calendar discovery, event-reading permission and event-writing permission distinct in API/UI state.
- Store account credentials separately in the OS vault. Preserve existing accounts when added-account consent fails or is cancelled. Verify reconnect, duplicate-account login, disconnect and restart behavior.
- Namespace provider identifiers and cache coverage by account/calendar/range. Preserve cached events on transient failures; distinguish a successful empty result from a failed fetch.
- Respect calendar roles and provider event restrictions. Use stable creation IDs, operation identity and conditional updates so retries avoid duplicates and stale writes cannot silently replace newer data.
- Keep UI optimism separate from confirmed provider state. Handle unknown write outcomes through reconciliation; do not blindly replay an uncertain write.
- Keep GraphQL field/argument documentation and exported schema aligned with implementation. Preserve authenticated introspection and actionable error responses.
- Follow `CONTRIBUTING.md` and `docs/telemetry.md` for diagnostic fields and remote allowlists. Never expose credentials, consent URLs, event bodies or personal account data in remote telemetry. Record meaningful actions and outcomes without dumping requests.
- Tests use mocks or isolated fixtures. Do not mutate the user's Google calendar or running cache/database to prove a test passes.
