# Web development

Use Bun 1.4.3 and Node 24 LTS. Run `bun install --frozen-lockfile`, start the Rust backend with `bash scripts/start.sh` (or the GPUI watcher), then run `bun run dev:web`. Open http://127.0.0.1:3000 for the landing page or http://127.0.0.1:3000/app for the calendar. Vite provides Fast Refresh. The production bundle is built with `bun run build`.

## Local connection

The development-only Vite bridge forwards same-origin POST requests to the loopback Axum backend. It reads `.local/session-token` on the server; the token is never included in browser assets. Host and Origin must both match `127.0.0.1:3000`. Keep Vite bound to loopback. Google OAuth runs against the existing desktop OAuth client and callback on this computer. Add/reconnect/disconnect accounts through Settings. No fake events are seeded.

The landing page and downloads are hosted at `https://cal.hibatillah.com`. The production `/app` route explains that account access is currently available only in the local preview; it does not attempt to connect to a visitor's local backend. Public calendar access requires user-scoped sessions, tenant isolation, a hosted database, encrypted provider credentials and web/mobile OAuth callbacks. Do not expose the desktop token or OS credential vault as a hosted authentication system.

## Vercel deployment

The Vercel project is `calendar` under `hibatillahs-projects`, with Root Directory `apps/web`, the TanStack Start preset, Node 24, install command `bun install --frozen-lockfile`, and build command `bun run build`. Shared workspace sources outside the app directory must remain available. No Google credentials, local session tokens, or root `.env` values belong in this frontend deployment.

The Nitro Vite adapter is enabled only when Vercel sets `VERCEL`; it produces Vercel functions and static assets. Ordinary local/CI builds keep their existing `dist/` output for GitHub release packaging. The custom subdomain uses a DNS-only Cloudflare CNAME pointing to the target provided by Vercel, with HTTPS managed by Vercel.

## Behavior

Week/month views, W/M/T shortcuts, Monday/Sunday week starts, sidebar calendar filters, profile/settings, event popover, create/edit, drag to move, and bottom-edge resize are available. Horizontal wheel or Shift+wheel advances by individual days. Today resets to the containing week. Times use Asia/Jakarta consistently with the current Rust backend; all-day ends remain exclusive. Editing recurring events changes only that occurrence.

Writes are optimistic and serialized in each client. Background refresh is paused during writes; errors restore the previous snapshot and preserve editor input. An identical failed editor retry retains its operation ID. There is no offline edit queue. Previously loaded ranges are stored in browser localStorage for offline viewing while the local web server is available; this is not an installable/offline-start PWA. Account disconnect clears snapshots. Settings can clear them manually. Activity contains static action descriptions, not event contents or credentials. Client PostHog integration is not yet added; the existing Rust backend continues its telemetry.

## Shared ownership and checks

- `packages/api`: GraphQL documents, generated operation types, transport and shared query/optimistic-write hooks. Generate from the committed Rust SDL with `bun run codegen`. Generated files are checked for drift in CI.
- `packages/calendar`: named-zone date math, month grids, day filtering and overlapping event placement. Temporal rejects ambiguous/nonexistent local times.
- `packages/design-tokens`: platform-neutral colors and font name. Web uses shadcn/Radix primitives and Tailwind; native clients own their UI.

Run `bun run typecheck`, `bun run test`, `bun run test:web`, and `bun run build`. Install Chromium once with `bunx playwright install chromium`. Browser tests intercept GraphQL; they must never create events in the real Google account. Native appearance is separate from these web checks.

Appearance in General settings supports Light, Dark, and System, follows operating-system changes, and persists in this browser. Calendar surfaces and portal menus/dialogs share theme tokens. The landing hero uses the light palette; native desktop and mobile themes are not changed by the web setting.
