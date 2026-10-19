# Calendar

A cross-platform Google Calendar app, vibe-coded with GPT-6 Astra.

Bring your Google calendars together, plan your week, and keep upcoming events close at hand. Calendar pairs a compact desktop calendar with a mobile month-and-agenda view and a web app.

## One calendar, across platforms

The desktop, web, and mobile apps work with the same Google accounts and calendars through a shared backend. Connect your Google account to access your schedule across platforms. Changes saved to Google Calendar are available to the other apps when they sync, so your events stay with your Google account.

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

Calendar is in active development. Windows desktop, web, and mobile previews are available for development and testing. macOS support is being prepared; iOS still needs physical-device verification. Public sign-up, signed releases, and offline editing are not ready yet.

The preview currently requires a local setup and your own Google OAuth configuration. See [Contributing](CONTRIBUTING.md) for setup, preview builds, and development instructions.

## License

[MIT](LICENSE). Bundled fonts and third-party assets retain their included licenses.
