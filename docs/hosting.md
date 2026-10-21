# Backend container preview

The frontend is on Vercel. The proposed backend deployment is GitHub Actions → GHCR → Coolify on an x86_64 server. This keeps Rust compilation off the credit-limited AWS machine and lets the same image move to another host.

**This image currently verifies infrastructure, not public calendar access.** The backend still has one local session token and one account collection, Desktop OAuth callbacks, and OS-vault credentials. It is not a multi-user service. Container mode disables Google connection configuration so credentials cannot silently fall back to a nonpersistent Linux vault. Do not assign a public domain or connect the Vercel app to it yet.

## Image and Coolify

`.github/workflows/backend-image.yml` builds Linux/amd64 on PRs into main and publishes only on pushes to main, after health, unauthenticated-request rejection, and graceful-stop checks. It uses `GITHUB_TOKEN`, not a personal token.

- Image: `ghcr.io/hibatillah/calendar-backend`
- Tags: `main` and immutable commit identifiers `sha-<full commit SHA>`
- Container port: `47821`
- Health check: `GET /health`
- Persistent volume: `/data`, writable by UID/GID `10001`
- Runtime: non-root Debian image; Google/telemetry secrets are not included

After the first publish, check the package visibility in GitHub Packages. Public images need no pull credential; a private package needs a read-only registry credential configured directly in Coolify. Prefer a commit tag for repeatable deployments and rollback.

In Coolify, use a Docker Compose resource with [deploy/compose.yml](../deploy/compose.yml), set `CALENDAR_BACKEND_IMAGE` to the published commit tag, and leave Domains empty. The host port binds only to `127.0.0.1`. Verify health through an SSH tunnel or on the server itself. No server access, DNS changes, or Coolify deployment is performed by this repository workflow.

Build locally with `docker build -f apps/backend/Dockerfile -t calendar-backend:preview .`. The root `.dockerignore` allows only build sources and bundled GraphiQL assets. Runtime state is created under `/data/.local`; the image contains neither the developer's `.env` nor their database.

## Before enabling the web calendar

1. Add browser sessions and tenant ownership to every account, event, cache, and mutation operation.
2. Implement Web OAuth with a fixed HTTPS callback, per-user state/PKCE, secure cookies, and the Google client configured for that callback.
3. Replace OS-vault token persistence with encrypted server storage and an externally managed encryption key.
4. Add PostgreSQL migrations, backups, restore tests, rate limits, and production CORS/CSRF controls.
5. Connect Vercel's server-side transport to the hosted API and verify real sign-in, reads, writes, sign-out, and account isolation.

Keep the desktop loopback transport intact during that work. A container must not make the desktop session token available to browser JavaScript.

## Leaving the trial server

Back up data to storage outside this AWS server and test restoration before the credits expire. Record the image commit tag, runtime configuration, and encrypted credential key separately from the database. The compose file caps this preview at one CPU and 512 MiB with rotated container logs. These limits do not cap AWS charges; configure billing alerts and plan a migration date.

References: [Coolify Docker images](https://coolify.io/docs/applications/deployments/docker-image), [GHCR](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry).
