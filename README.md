# Calendar

A lightweight Google Calendar client for Windows, built with Rust and GPUI.

Bring your Google calendars together, plan your week, and keep upcoming events close at hand from the system tray. The app talks to Google Calendar directly from your machine; there is no server component.

[Releases](https://github.com/hibatillah/calendar/releases)

## What you can do

- Connect multiple Google accounts and choose which calendars to display.
- Browse weeks and months, or scroll freely across dates.
- Create and edit events; drag or resize events with immediate visual feedback.
- View previously loaded events offline.
- Check upcoming events from the Windows tray.

## Project status

Calendar is in active development and Windows is the only supported platform. It installs two small programs: `calendar-tray.exe` (tray icon and background sync, always running) and `calendar-desktop.exe` (the window, which exits when closed). Sync and event editing are tested against fake servers only and have not yet been verified with a real Google account; see [architecture](docs/architecture.md).

Sign-in needs a Google OAuth client compiled into the app; to build with your own, follow [setup](docs/google.md). Releases are unsigned. See [Contributing](CONTRIBUTING.md) for build, test and packaging instructions.

## License

[MIT](LICENSE). Bundled fonts retain their included licenses.
