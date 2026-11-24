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

Calendar is in active development and Windows is the only supported platform. The app is being moved from a local-backend design to direct Google Calendar access; see [architecture](docs/architecture.md) for the current state, the target layout and the migration steps that remain.

The preview requires your own Google OAuth client ([setup](docs/google.md)). Releases are unsigned. See [Contributing](CONTRIBUTING.md) for build, test and packaging instructions.

## License

[MIT](LICENSE). Bundled fonts retain their included licenses.
