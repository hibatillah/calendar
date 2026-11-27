---
paths:
  - "crates/desktop/src/service.rs"
  - "crates/calendar-core/**/*.rs"
---

# Google and persistence

- Read `docs/google.md` before changing OAuth or provider calls. Keep calendar discovery, event-reading permission and event-writing permission distinct in API/UI state.
- Store account credentials separately in the OS vault. Preserve existing accounts when added-account consent fails or is cancelled. Verify reconnect, duplicate-account login, disconnect and restart behavior.
- Namespace provider identifiers and cache coverage by account/calendar/range. Preserve cached events on transient failures; distinguish a successful empty result from a failed fetch.
- Respect calendar roles and provider event restrictions. Use stable creation IDs, operation identity and conditional updates so retries avoid duplicates and stale writes cannot silently replace newer data.
- Keep UI optimism separate from confirmed provider state. Handle unknown write outcomes through reconciliation; do not blindly replay an uncertain write.
- Nothing leaves the machine except calls to Google. Never write credentials, consent URLs, event bodies or personal account data to logs or error messages.
- Tests use mocks or isolated fixtures. Do not mutate the user's Google calendar or running cache/database to prove a test passes.
