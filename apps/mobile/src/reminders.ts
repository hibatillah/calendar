import type { CalendarEvent } from "@calendar/calendar";
export async function enableReminders() {
  return false;
}
export async function syncReminders(
  _events: CalendarEvent[],
  _minutes: number | null,
) {}
export function watchReminderOpen(_onOpen: (date: string) => void) {
  return () => {};
}
