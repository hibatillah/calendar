# Landing page

Run `bun run dev:web` and open http://127.0.0.1:3000. The calendar client is at `/app`, platform availability at `/download`, and the preview data explanation at `/privacy`.

The server-rendered landing uses bundled Inter, shadcn/Radix buttons and popovers, and scoped CSS. Its rounded sky-colored hero and spacious product presentation take inspiration from aside.com; all calendar illustrations are original DOM/CSS with fictional events. The interactive week/month demo never accesses GraphQL or the user's calendars. Navigation and demo buttons become enabled after hydration.

Content order: hero and interactive desktop preview, three feature cards, mobile month/agenda preview, tray preview, data handling, platform availability, FAQ, footer. Mobile and macOS work is explicitly labeled in development. The download section reads published preview releases and links directly to existing assets. Windows and Android are developer previews; iOS archives are explicitly simulator-only, and macOS remains unavailable. See CONTRIBUTING.md for the release pipeline, setup requirements, and signing limitations.

The marketing routes can be hosted separately from local backend access. `/app` is still a local client: public sessions, tenant isolation, hosted OAuth callbacks and server persistence are required before public sign-in. Recommended deployment direction is TanStack Start on Vercel and the existing Axum service on a managed container host such as Render. Ordinary edge functions are not a drop-in host for this native Rust/SQLite process.

Validation: `bun run typecheck`, `bun run test:web`, `bun run build`. Landing browser checks cover demo interaction, mobile navigation, overflow, availability/privacy routes and absence of calendar API requests. Existing calendar browser tests use `/app` and mocked GraphQL.
