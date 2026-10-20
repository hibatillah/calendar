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
      <main className="marketing-page">
        <MarketingNav />
        <article className="privacy-document">
          <span className="section-eyebrow">WEB APP PREVIEW</span>
          <h1>Your calendar, soon on the web.</h1>
          <p>
            The web calendar currently runs with the local Calendar backend. Google
            sign-in and calendar syncing are not available on this hosted site yet.
          </p>
          <p>
            Download the desktop preview to connect your Google accounts, or run the web
            app locally using the setup guide.
          </p>
          <a href="/download" className="marketing-inline-link">
            Download Calendar →
          </a>
          <p>
            <a
              href="https://github.com/hibatillah/calendar/blob/main/docs/web.md"
              className="marketing-inline-link"
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
