import { dateKey, type CalendarEvent } from "@calendar/calendar";

export function reminderPlan(
  events: CalendarEvent[],
  minutes: number | null,
  now = Date.now(),
) {
  if (minutes === null) return [];

  return events
    .filter((event) => !event.allDay)
    .map((event) => ({
      identifier: `calendar:${event.eventId}`,
      eventId: event.eventId,
      date: dateKey(new Date(event.start)),
      fireAt: Date.parse(event.start) - minutes * 60_000,
    }))
    .filter((reminder) => reminder.fireAt > now)
    .sort(
      (first, second) =>
        first.fireAt - second.fireAt || first.eventId.localeCompare(second.eventId),
    )
    .slice(0, 50);
}
