import { createFileRoute } from "@tanstack/react-router";
import { MarketingFooter, MarketingNav } from "../components/landing-page";
export const Route = createFileRoute("/privacy")({
  component: Privacy,
  head: () => ({ meta: [{ title: "Preview privacy · Calendar" }] }),
});
function Privacy() {
  return (
    <main className="marketing-page">
      <MarketingNav />
      <article className="privacy-document">
        <span className="section-eyebrow">
          DEVELOPER PREVIEW · OCTOBER 2026
        </span>
        <h1>
          Your calendar data
          <br />
          in this preview.
        </h1>
        <p>
          This describes the current local preview. A hosted public service will
          need updated privacy information before launch.
        </p>
        <h2>Google Calendar</h2>
        <p>
          You grant calendar access through Google. The local Rust backend uses
          that access to read and save your events. Disconnecting an account
          removes the local Google credentials and attempts to revoke the grant.
        </p>
        <h2>On your devices</h2>
        <p>
          The desktop backend and clients cache calendar information for offline
          viewing. The mobile app stores its connection token in the operating
          system’s secure storage. Browser previews cache loaded calendar ranges
          on that browser. Do not use a shared device for private calendars.
        </p>
        <h2>Diagnostics and analytics</h2>
        <p>
          The desktop has local diagnostic logs. Optional PostHog reporting is
          controlled by configuration; logs and telemetry exclude event content
          and credentials. The landing page does not load third-party analytics
          or tracking scripts.
        </p>
        <h2>Downloads and feedback</h2>
        <p>
          Preview downloads and issue reports are hosted on GitHub and follow
          GitHub’s policies. Do not post tokens, private calendar screenshots,
          or event details in public issues.
        </p>
        <a
          href="https://github.com/hibatillah/calendar"
          className="marketing-inline-link"
        >
          Read the source and documentation <ArrowLink />
        </a>
      </article>
      <MarketingFooter />
    </main>
  );
}
function ArrowLink() {
  return <span aria-hidden="true">↗</span>;
}
