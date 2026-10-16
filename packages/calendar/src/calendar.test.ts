import { describe, it, expect } from "vitest";
import {
  addDays,
  dateKey,
  eventsOnDay,
  localInstant,
  monthDays,
  startOfWeek,
  timedPlacement,
} from "./index";

const event = (eventId: string, start: string, end: string, allDay = false) => ({
  eventId,
  calendarId: "account|calendar",
  title: "Fixture",
  start,
  end,
  allDay,
  color: 123,
  readOnly: false,
});

describe("calendar dates", () => {
  it("Today selects the containing week for Monday and Sunday preferences", () => {
    expect(startOfWeek("2026-10-15", 1)).toBe("2026-10-12");
    expect(startOfWeek("2026-10-15", 0)).toBe("2026-10-11");
    expect(startOfWeek("2026-10-18", 1)).toBe("2026-10-12");
  });
  it("includes leap day and complete month weeks", () => {
    const dates = monthDays("2024-02-15");

    expect(dates).toContain("2024-02-29");
    expect(dates.length % 7).toBe(0);
    expect(addDays("2024-02-28", 1)).toBe("2024-02-29");
    expect(monthDays("2026-03-01").length).toBe(42);
  });
  it("places instants using display zone, independent of machine zone", () => {
    expect(dateKey(new Date("2026-10-14T18:00:00Z"))).toBe("2026-10-15");
  });
  it("rejects both DST gaps and repeated local times", () => {
    expect(() => localInstant("2026-03-08", "02:30", "America/New_York")).toThrow();
    expect(() => localInstant("2026-11-01", "01:30", "America/New_York")).toThrow();
  });
  it("preserves exclusive all-day ends and sorts all-day before timed events", () => {
    const events = [
      event(
        "timed",
        localInstant("2026-10-15", "08:00"),
        localInstant("2026-10-15", "09:00"),
      ),
      event(
        "all-day",
        localInstant("2026-10-14", "00:00"),
        localInstant("2026-10-16", "00:00"),
        true,
      ),
    ];

    expect(eventsOnDay(events, "2026-10-15").map((event) => event.eventId)).toEqual([
      "all-day",
      "timed",
    ]);
    expect(eventsOnDay(events, "2026-10-16")).toEqual([]);
  });
  it("clips cross-midnight events and overlaps concurrent lanes slightly", () => {
    const events = [
      event(
        "first",
        localInstant("2026-10-14", "23:00"),
        localInstant("2026-10-15", "02:00"),
      ),
      event(
        "second",
        localInstant("2026-10-15", "01:00"),
        localInstant("2026-10-15", "03:00"),
      ),
    ];
    const positions = timedPlacement(events, "2026-10-15");

    expect(positions[0].top).toBe(0);
    expect(positions[0].height).toBe(120);
    expect(positions[0].width).toBeGreaterThan(positions[1].left);
  });
});
