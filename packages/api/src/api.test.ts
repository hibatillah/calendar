import { afterEach, expect, it, vi } from "vitest";
import {
  CalendarSnapshotDocument,
  createTransport,
  type CalendarSnapshotQuery,
} from "./index";
import { replaceOptimisticEvent } from "./react";
afterEach(() => vi.unstubAllGlobals());
it("surfaces GraphQL failures rather than treating HTTP 200 as success", async () => {
  vi.stubGlobal(
    "fetch",
    vi
      .fn()
      .mockResolvedValue(
        new Response(JSON.stringify({ errors: [{ message: "READ_ONLY" }] })),
      ),
  );
  await expect(
    createTransport("/graphql")(CalendarSnapshotDocument, {
      startDate: "2026-10-12",
      endDate: "2026-10-19",
    }),
  ).rejects.toThrow("READ_ONLY");
});
it("replaces an optimistic event by identity instead of duplicating it", () => {
  const event = {
    eventId: "event",
    calendarId: "account|calendar",
    title: "Fixture",
    start: "2026-10-15T01:00:00Z",
    end: "2026-10-15T02:00:00Z",
    timezone: "Asia/Jakarta",
    allDay: false,
    color: 123,
    readOnly: false,
    revision: 1,
    description: "",
    location: "",
  };
  const snapshot: CalendarSnapshotQuery = {
    events: [event],
    googleConnection: {
      configured: true,
      connected: true,
      connecting: false,
      message: "",
      accounts: [],
    },
  };
  const updated = replaceOptimisticEvent(snapshot, {
    ...event,
    start: "2026-10-16T01:00:00Z",
  });
  expect(updated.events).toHaveLength(1);
  expect(updated.events[0].start).toContain("2026-10-16");
  expect(snapshot.events[0].start).toContain("2026-10-15");
});
