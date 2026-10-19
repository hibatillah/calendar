import { describe, expect, it } from "vitest";
import { reminderPlan } from "./reminder-plan";
const event = { eventId: "event", calendarId: "calendar", title: "Private", start: "2026-10-16T10:00:00Z", end: "2026-10-16T11:00:00Z", allDay: false, color: 0, readOnly: false };
describe("local reminder plan", () => {
  it("excludes past and all-day reminders and keeps titles out of notification data", () => {
    const now = Date.parse("2026-10-16T09:49:00Z");
    const plan = reminderPlan([event, { ...event, eventId: "all-day", allDay: true }], 10, now);
    expect(plan).toEqual([{ identifier: "calendar:event", eventId: "event", date: "2026-10-16", fireAt: Date.parse("2026-10-16T09:50:00Z") }]);
    expect(reminderPlan([event], 10, now + 60_000)).toEqual([]);
    expect(reminderPlan([event], null, now)).toEqual([]);
  });
  it("caps pending requests below the iOS limit and reschedules moved events", () => {
    const events = Array.from({ length: 80 }, (_, index) => ({ ...event, eventId: `event-${index}` }));
    expect(reminderPlan(events, 0, 0)).toHaveLength(50);
    expect(reminderPlan([{ ...event, start: "2026-10-17T10:00:00Z" }], 10, 0)[0].fireAt).not.toBe(reminderPlan([event], 10, 0)[0].fireAt);
  });
});
