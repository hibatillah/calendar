import { createFileRoute } from "@tanstack/react-router";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";
import { CalendarApp } from "../components/calendar-app";

export const Route = createFileRoute("/app")({
  component: AppPage,
  ssr: false,
  head: () => ({ meta: [{ title: "Your calendar · Calendar" }] }),
});

function AppPage() {
  const [client] = useState(() => new QueryClient());

  return (
    <QueryClientProvider client={client}>
      <CalendarApp />
    </QueryClientProvider>
  );
}
