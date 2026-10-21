import { marketingPageClasses } from "@/lib/marketing-styles";
import { createFileRoute } from "@tanstack/react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { CalendarApp } from "../components/calendar-app";
import { MarketingFooter, MarketingNav } from "../components/landing-page";

export const Route = createFileRoute("/app")({
  component: AppPage,
  ssr: false,
  head: () => ({ meta: [{ title: "Your calendar · Calendar" }] }),
});

function AppPage() {
  const [client] = useState(() => new QueryClient());

  if (import.meta.env.PROD) {
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
            WEB APP PREVIEW
          </span>
          <h1>Your calendar, soon on the web.</h1>
          <p>
            The web calendar currently runs with the local Calendar backend. Google
            sign-in and calendar syncing are not available on this hosted site yet.
          </p>
          <p>
            Download the desktop preview to connect your Google accounts, or run the web
            app locally using the setup guide.
          </p>
          <a
            href="/download"
            className="marketing-inline-link inline-flex gap-2 items-center text-[12px] [border-bottom:1px_solid_#a4afa0] pb-1"
          >
            Download Calendar →
          </a>
          <p>
            <a
              href="https://github.com/hibatillah/calendar/blob/main/docs/web.md"
              className="marketing-inline-link inline-flex gap-2 items-center text-[12px] [border-bottom:1px_solid_#a4afa0] pb-1"
            >
              Local web setup ↗
            </a>
          </p>
        </article>
        <MarketingFooter />
      </main>
    );
  }

  return (
    <QueryClientProvider client={client}>
      <CalendarApp />
    </QueryClientProvider>
  );
}
