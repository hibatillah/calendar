# Desktop presentation

- Use bundled Inter for the calendar, inputs, popovers and settings. Keep the UI compact, with a gray sidebar (`#222222`) and near-black main surface (`#191919`). Avoid purple tint and transparency unless requested.
- Extend the sidebar through the header; keep the header seamless with no horizontal separator or branding label. Keep titlebar controls compact.
- Use proper vector icons with explicit contrasting colors and usable hit areas. Account/profile controls must not overlap the sidebar-reopen button or native drag regions.
- Group sidebar calendars by Google account. Keep account connection, consent and disconnect controls in Settings; keep Activity at the sidebar bottom.
- Put the profile control above the main month title. Its menu exposes identity, Settings and logout; use a clear fallback when a photo is unavailable.
- Use a non-modal event popover that leaves the calendar usable. Use a wide settings dialog with left navigation and right content. Both must isolate scrolling and pointer events from the calendar underneath.
- Start the main window at its configured minimum size, centered and windowed. Verify layout at that size as well as after resizing.
- Preserve Windows tray behavior and macOS titlebar/menu-bar boundaries. Report platform verification separately.
