# Landing page

Run `bun run dev:web` and open http://127.0.0.1:3000. The calendar client is at `/app`, platform availability at `/download`, and the preview data explanation at `/privacy`.

The server-rendered landing uses bundled Inter, shadcn/Radix buttons and popovers, and scoped CSS. Its rounded sky-colored hero and spacious product presentation take inspiration from aside.com; all calendar illustrations are original DOM/CSS with fictional events. The interactive week/month demo never accesses GraphQL or the user's calendars. Navigation and demo buttons become enabled after hydration.

Content order: hero and interactive desktop preview, three feature cards, mobile month/agenda preview, tray preview, data handling, platform availability, FAQ, footer. Platform cards show Windows (MSI), Android (APK), and Web (the `/app` route). Apple builds remain in CI and any simulator archives remain in GitHub Releases, but neither Apple platform is advertised as a download card. The Windows card falls back to a clearly labeled portable ZIP when the latest release has no MSI, and prefers MSI as soon as one is published. Preview and production use the same public release assets.

The header animates into a floating translucent pill after scrolling, with extra left padding around the wordmark and reduced-motion support. The hero title stays on one line, the taller desktop preview is cropped to its top 80% and is clipped inside the hero background, and the straight phone preview shows approximately its top two thirds. Tailwind utilities style every web view, including marketing layouts, decorative gradients and fictional product illustrations. `styles.css` is the only stylesheet, containing Tailwind, Inter, light/dark theme tokens and base defaults. Runtime event dimensions and provider colors enter through CSS custom properties consumed by utilities.

The marketing routes are hosted on Vercel. Public sessions, tenant isolation, hosted OAuth callbacks and server persistence are required before public sign-in. See [hosting](hosting.md) for the GHCR/Coolify container preview and the remaining backend work.

Validation: `bun run typecheck`, `bun run test:web`, `bun run build`. Landing browser checks cover demo interaction, mobile navigation, overflow, availability/privacy routes and absence of calendar API requests. Existing calendar browser tests use `/app` and mocked GraphQL.
