import { marketingPageClasses } from "@/lib/marketing-styles";
import { createFileRoute } from "@tanstack/react-router";
import { MarketingFooter, MarketingNav } from "../components/landing-page";

export const Route = createFileRoute("/privacy")({
  component: Privacy,
  head: () => ({ meta: [{ title: "Preview privacy · Calendar" }] }),
});

function Privacy() {
  return (
    <main className={marketingPageClasses}>
      <MarketingNav />
      <article
        className={`
  privacy-document max-w-175 m-[75px_auto_100px] [&_h1]:text-[54px] [&_h1]:tracking-[-2.5px]
  [&_h1]:mb-7.5 [&_p]:text-[#788171] [&_p]:m-[14px_0_30px] [&_h2]:text-[26px] [&_h2]:tracking-[-0.8px]
  [&_h2]:mt-10 max-[700px]:m-[45px_22px_70px] max-[700px]:[&_h1]:text-[42px]
`}
      >
        <span
          className={`
  section-eyebrow text-[9px] font-[550] tracking-[1.7px] block text-[#718078] mb-[23px]
  [.privacy-strip_&]:mb-[15px] max-[700px]:text-[8px] max-[700px]:tracking-[1.3px] max-[700px]:mb-4.5
`}
        >
          DEVELOPER PREVIEW · OCTOBER 2026
        </span>
        <h1>
          Your calendar data
          <br />
          in this preview.
        </h1>
        <p>
          This describes the current local preview. A hosted public service will need
          updated privacy information before launch.
        </p>
        <h2>Google Calendar</h2>
        <p>
          You grant calendar access through Google. The local Rust backend uses that
          access to read and save your events. Disconnecting an account removes the
          local Google credentials and attempts to revoke the grant.
        </p>
        <h2>On your devices</h2>
        <p>
          The desktop backend and clients cache calendar information for offline
          viewing. The mobile app stores its connection token in the operating system’s
          secure storage. Browser previews cache loaded calendar ranges on that browser.
          Do not use a shared device for private calendars.
        </p>
        <h2>Diagnostics and analytics</h2>
        <p>
          The desktop has local diagnostic logs. Optional PostHog reporting is
          controlled by configuration; logs and telemetry exclude event content and
          credentials. The landing page does not load third-party analytics or tracking
          scripts.
        </p>
        <h2>Downloads and feedback</h2>
        <p>
          Preview downloads and issue reports are hosted on GitHub and follow GitHub’s
          policies. Do not post tokens, private calendar screenshots, or event details
          in public issues.
        </p>
        <a
          href="https://github.com/hibatillah/calendar"
          className="marketing-inline-link inline-flex gap-2 items-center text-[12px] [border-bottom:1px_solid_#a4afa0] pb-1"
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
