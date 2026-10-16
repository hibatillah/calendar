import { createFileRoute } from "@tanstack/react-router";
import { LandingPage } from "../components/landing-page";

export const Route = createFileRoute("/")({
  component: LandingPage,
  head: () => ({
    meta: [
      { title: "Calendar — Your week, clearly." },
      {
        name: "description",
        content:
          "A simple home for your Google calendars. Plan your week on desktop, see your day on mobile, and keep what’s next close at hand.",
      },
      { property: "og:title", content: "Calendar — Your week, clearly." },
      {
        property: "og:description",
        content:
          "Your Google calendars, together in one calm space. Explore the Calendar preview.",
      },
    ],
  }),
});
