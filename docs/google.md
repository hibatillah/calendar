# Google account connection

Calendar connects multiple Google accounts and groups calendars by account. Writable calendars support creation, editing, moving and resizing events. Reader calendars and special Google event types remain read-only. Sample data is removed on startup.

## Configure once

1. In [Google Cloud](https://console.cloud.google.com/), create/select a project and enable **Google Calendar API**.
2. Configure **Google Auth Platform** branding/audience. For personal testing, keep the external app in Testing and add your Google email as a test user.
3. Create an OAuth client with application type **Desktop app** (not Web application). Copy its client ID and client secret into the repository-root `.env`:

```dotenv
GOOGLE_CLIENT_ID=your-client-id.apps.googleusercontent.com
GOOGLE_CLIENT_SECRET=your-desktop-client-secret
```

Keep the existing PostHog settings. Do not commit `.env` or paste credentials in chat. Desktop OAuth clients are public clients; the client secret is a Google configuration requirement, not proof of app identity. PKCE protects the authorization code.

4. Restart with `bash scripts/start.sh`. Open the profile circle, then **Settings → Google accounts → Add Google account**. Complete consent in your browser.

The callback binds a temporary random port on `127.0.0.1`; Desktop clients support this without manually configuring a web redirect URI. Sign-in expires after five minutes. **Cancel sign-in** clears the pending flow. **Disconnect Google** removes the local credential and attempts to revoke the grant; if Google is unreachable the app explains how to revoke access in your Google account.

## Credentials and permissions

The Calendar rename migrates existing Google credentials into `calendar.google.local` on first access. Local caches, account-slot identifiers and preferences keep their existing paths. Old provider operation markers remain readable for retry reconciliation; new writes use `calendarOperation`. If Google still displays an older consent-screen name, update the OAuth project's branding in Google Cloud Console separately.

The requested scopes are `calendar.calendarlist.readonly` and `calendar.events` under `https://www.googleapis.com/auth/`, plus `openid email profile` for the name and avatar. Existing accounts must choose **Approve calendar access** in Settings and complete browser consent. A timed-out or denied consent preserves the existing connection.

Refresh tokens go into Windows Credential Manager under service `calendar.google.local`, with separate entries per account (the legacy entry is `desktop-account`); macOS uses Keychain. Access tokens live in memory. Tokens are excluded from SQLite, `.env`, logs and analytics. `.local/google-accounts.json` stores only credential-slot identifiers; profile images are cached in `.local/avatars`.

Google testing-mode grants may expire after seven days; reconnect if the app reports that authorization expired or was revoked. Public distribution requires Google consent-screen/verification work appropriate to the scopes requested.

## API and validation

Authenticated GraphQL provides `googleConnection`, `connectGoogle`, and `disconnectGoogle`; see the local GraphiQL Docs panel. The callback is a separate short-lived loopback HTTP listener protected by a one-use random state and S256 PKCE. Do not expose the local backend publicly.

Tests cover the RFC PKCE vector, wrong-state rejection, one-use denial handling, duplicate sign-in prevention, and a real loopback browser callback with simulated denial. Live consent, credential-vault persistence, and Google calendar discovery must be verified with your configured Google project and account.

References: [Google Desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app), [Calendar list API](https://developers.google.com/workspace/calendar/api/v3/reference/calendarList/list).

## Event sync and offline viewing

The backend loads paginated events with `singleEvents=true`, expanding recurring instances within a bounded date range. The desktop requests a 56-day window around the displayed month and refreshes every 30 seconds while idle. Navigating to another month loads its cache, then refreshes Google. Cached events are isolated by account and month; disconnect clears them. Failed refreshes retain cached events and show an error. All-day end dates remain exclusive; date-only values map to midnight in the current fixed Asia/Jakarta display zone. Incremental sync tokens and configurable display zones are future work.

## Writes and preferences

Creates use a stable provider event ID; an operation marker reconciles duplicate retries. Updates check the current revision and PATCH with `If-Match`. Concurrent changes require a refresh. Editing an expanded recurring event changes that occurrence only. Existing guests receive Google update notifications (`sendUpdates=all`); attendee management and series-wide editing are not exposed. Offline editing remains disabled. Mock-provider tests cover creation retries, edits and stale-version rejection.

Settings include Monday/Sunday week start, account profiles, tray all-day visibility and Google account management. T/W/M navigate to the current week, week view and month view. Preferences live in `.local/preferences.json`. The desktop starts at its 1050 × 680 minimum size and uses bundled Inter throughout.
