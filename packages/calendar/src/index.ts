import { Temporal } from "@js-temporal/polyfill";

export const DISPLAY_ZONE = "Asia/Jakarta";

export type WeekStart = 0 | 1;

export type CalendarEvent = {
  eventId: string;
  calendarId: string;
  title: string;
  start: string;
  end: string;
  allDay: boolean;
  color: number;
  readOnly: boolean;
};

export function dateKey(value = new Date(), zone = DISPLAY_ZONE): string {
  return Temporal.Instant.from(value.toISOString())
    .toZonedDateTimeISO(zone)
    .toPlainDate()
    .toString();
}

export function addDays(date: string, amount: number): string {
  return Temporal.PlainDate.from(date).add({ days: amount }).toString();
}

export function addMonths(date: string, amount: number): string {
  return Temporal.PlainDate.from(date).add({ months: amount }).toString();
}

export function startOfWeek(date: string, weekStart: WeekStart = 1): string {
  const weekday = Temporal.PlainDate.from(date).dayOfWeek % 7;

  return addDays(date, -((weekday - weekStart + 7) % 7));
}

export function monthDays(date: string, weekStart: WeekStart = 1): string[] {
  const month = Temporal.PlainDate.from(date).with({ day: 1 });
  const first = startOfWeek(month.toString(), weekStart);
  const offset = month.since(Temporal.PlainDate.from(first)).days;

  return Array.from(
    { length: Math.ceil((offset + month.daysInMonth) / 7) * 7 },
    (_, index) => addDays(first, index),
  );
}

export function localInstant(date: string, time: string, zone = DISPLAY_ZONE): string {
  return Temporal.PlainDateTime.from(`${date}T${time}`)
    .toZonedDateTime(zone, { disambiguation: "reject" })
    .toInstant()
    .toString();
}

export function eventsOnDay<Event extends CalendarEvent>(
  events: Event[],
  date: string,
): Event[] {
  const start = Date.parse(localInstant(date, "00:00"));
  const end = Date.parse(localInstant(addDays(date, 1), "00:00"));

  return events
    .filter((event) => Date.parse(event.start) < end && Date.parse(event.end) > start)
    .sort(
      (first, second) =>
        Number(second.allDay) - Number(first.allDay) ||
        Date.parse(first.start) - Date.parse(second.start) ||
        first.eventId.localeCompare(second.eventId),
    );
}

export function clockTime(instant: string): string {
  const time = Temporal.Instant.from(instant).toZonedDateTimeISO(DISPLAY_ZONE);

  return `${String(time.hour).padStart(2, "0")}:${String(time.minute).padStart(2, "0")}`;
}

export function monthLabel(date: string): string {
  return new Date(`${date}T12:00:00Z`).toLocaleDateString("en", {
    month: "long",
    year: "numeric",
    timeZone: "UTC",
  });
}

export function dayLabel(date: string): string {
  return `${new Date(`${date}T12:00:00Z`).toLocaleDateString("en", { weekday: "short", timeZone: "UTC" })} ${Number(date.slice(-2))}`;
}

export function eventColor(color: number): string {
  return `#${color.toString(16).padStart(6, "0")}`;
}

export function timedPlacement(events: CalendarEvent[], date: string) {
  const timedEvents = eventsOnDay(events, date).filter((event) => !event.allDay);
  const clusters: CalendarEvent[][] = [];
  let clusterEnd = 0;

  for (const event of timedEvents) {
    if (Date.parse(event.start) >= clusterEnd) {
      clusters.push([]);
      clusterEnd = 0;
    }

    clusters[clusters.length - 1].push(event);
    clusterEnd = Math.max(clusterEnd, Date.parse(event.end));
  }

  return clusters.flatMap((cluster) => {
    const laneEnds: number[] = [];
    const placements = cluster.map((event) => {
      let lane = laneEnds.findIndex((end) => end <= Date.parse(event.start));

      if (lane < 0) lane = laneEnds.length;

      laneEnds[lane] = Date.parse(event.end);

      return { event, lane };
    });

    return placements.map(({ event, lane }) => {
      const startDate = dateKey(new Date(event.start));
      const endDate = dateKey(new Date(event.end));
      const minutes = (instant: string) => {
        const [hour, minute] = clockTime(instant).split(":").map(Number);

        return hour * 60 + minute;
      };
      const startMinute = startDate < date ? 0 : minutes(event.start);
      const endMinute = endDate > date ? 1440 : minutes(event.end);
      const stride = 100 / laneEnds.length;

      return {
        event,
        top: startMinute,
        height: Math.max(18, endMinute - startMinute),
        left: lane * stride,
        width: Math.min(100 - lane * stride, stride + (laneEnds.length > 1 ? 5 : 0)),
      };
    });
  });
}
