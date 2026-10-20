# Calendar

A cross-platform Google Calendar app, vibe-coded with GPT-6 Astra.

Bring your Google calendars together, plan your week, and keep upcoming events close at hand. Calendar pairs a compact desktop calendar with a mobile month-and-agenda view and a web app.

[Website](https://cal.hibatillah.com) · [Downloads](https://cal.hibatillah.com/download) · [Releases](https://github.com/hibatillah/calendar/releases)

## One calendar, across platforms

The desktop, web, and mobile apps share a backend for Google accounts and calendars. Changes saved to Google Calendar are available to the other apps when they sync, so your events stay with your Google account. Cross-platform access currently requires a development setup; hosted sign-in is still being built.

| App     | Experience                                                                                                                   | Built with                            |
| ------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------- |
| Desktop | A native week/month calendar with drag-and-drop editing and a tray agenda. Windows first, with macOS support being prepared. | Rust and GPUI                         |
| Web     | A browser calendar with week/month views, plus the project website.                                                          | TypeScript, React, and TanStack Start |
| Mobile  | A month grid above the selected day's event list, designed for Android and iOS.                                              | TypeScript, React Native, and Expo    |

All three apps use the same Rust/Axum GraphQL backend for Google account connections and calendar data. Web and mobile also share calendar helpers and a typed API client. The current preview uses a locally running backend; a hosted account experience is still in development.

## What you can do

- Connect multiple Google accounts and choose which calendars to display.
- Browse weeks and months, or scroll freely across dates on desktop.
- Create and edit events; drag or resize desktop events with immediate visual feedback.
- View previously loaded events offline.
- Check upcoming events from the Windows tray.
- On mobile, select a day in the month grid to see its events in an ordered agenda.

## Project status

Calendar is in active development. The website is live, with desktop, mobile, and tray previews. The download page presents Windows, Android, and Web and reads available application files from the latest GitHub release.

- **Windows:** MSI packaging installs the desktop app and its local backend together, with shortcuts and uninstall support. Configuration and cached calendars are kept separately from the installation. The MSI becomes downloadable once a release includes it; existing portable ZIP downloads remain available.
- **Android:** preview APKs are distributed through GitHub releases.
- **Web:** the landing page and browser calendar live in the same TanStack Start app on Vercel. The public calendar route explains the current setup limitation; hosted Google sign-in is not available yet.
- **Apple platforms:** macOS build checks and iOS simulator builds remain in CI, but neither platform is advertised as a public download. Native macOS testing and physical iOS device verification are still needed.

Backend container packaging is being prepared for GHCR and Coolify. This is an infrastructure preview, not a public account service: hosted authentication and per-user data isolation must be implemented before connecting it to the live web app.

The calendar preview currently requires a local setup and your own Google OAuth configuration. Public sign-up, signed releases, and offline editing are not ready yet. See [Contributing](CONTRIBUTING.md) for setup, testing, and packaging, and the [hosting guide](docs/hosting.md) for backend deployment preparation.

## License

[MIT](LICENSE). Bundled fonts and third-party assets retain their included licenses.
