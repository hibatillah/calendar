---
paths:
  - "apps/desktop/src/**/*.rs"
  - "apps/backend/src/**/*.rs"
  - "crates/calendar-core/**/*.rs"
  - "crates/calendar-layout/**/*.rs"
---

# Calendar behavior

- Today opens the week containing today, using the configured Monday/Sunday start. Free horizontal scrolling can show any consecutive date range; it must remain independent of vertical time scrolling.
- Derive sidebar highlights from the displayed dates, not a fixed calendar week. A clipped edge must not introduce an extra highlighted day. Apply week-start preferences consistently to sidebar and month grids.
- Place moved/resized events optimistically at the target and retain them while saving. Keep the saving indicator; restore the prior position only on failure. Background snapshots must not overwrite pending edits.
- Keep a new draft visible while its editor is open. Reconcile its temporary identifier with the saved provider identifier without duplicate cards. Failed saves must preserve a recoverable draft and explain the failure.
- Preserve account and calendar identity through selection, cache, layout, mutations and reconciliation, including calendars shared between accounts.
- Preserve exclusive end dates for all-day events and correct local-day placement across midnight and time-zone boundaries. Reject invalid or ambiguous local times explicitly.
- Treat recurring occurrences and recurring series as distinct edit scopes. State which scope is supported before a write; preserve provider instance identity.
- Permission failures and offline state need actionable feedback. Creating an event must not silently do nothing or pretend to save locally when Google has not accepted it.
