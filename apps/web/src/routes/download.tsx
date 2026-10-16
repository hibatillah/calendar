import { createFileRoute } from "@tanstack/react-router";
import {
  DownloadSection,
  MarketingFooter,
  MarketingNav,
} from "../components/landing-page";

export const Route = createFileRoute("/download")({
  component: Downloads,
  head: () => ({ meta: [{ title: "Download · Calendar" }] }),
});

function Downloads() {
  return (
    <main className="marketing-page download-page">
      <MarketingNav />
      <DownloadSection />
      <MarketingFooter />
    </main>
  );
}
