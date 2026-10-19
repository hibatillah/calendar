import { clockTime, eventColor, eventsOnDay, monthLabel } from "@calendar/calendar";
import type { CalendarController } from "./use-calendar-controller";

export function MonthCalendar({ state }: { state: CalendarController }) {
  const {
    today,
    selectedDate,
    setSelectedDate,
    setFirstDay,
    setView,
    monthDates,
    events,
    newEvent,
    openEvent,
  } = state;

  return (
    <div
      className="month-grid"
      style={{
        gridTemplateRows: `repeat(${monthDates.length / 7}, minmax(110px,1fr))`,
      }}
    >
      {monthDates.map((date) => (
        <div className="month-cell" key={date} onDoubleClick={() => newEvent(date)}>
          <button
            className={`block ml-auto px-1.5 py-1 mb-1 ${date === today ? "today" : ""} ${date.slice(0, 7) !== selectedDate.slice(0, 7) ? "muted" : ""}`}
            onClick={() => {
              setSelectedDate(date);
              setFirstDay(date);
              setView("week");
            }}
          >
            {date.endsWith("-01")
              ? monthLabel(date).split(" ")[0] + " 1"
              : Number(date.slice(-2))}
          </button>
          {eventsOnDay(events, date).map((event) => (
            <button
              key={event.eventId}
              onClick={() => openEvent(event)}
              className="month-event"
              style={{ borderColor: eventColor(event.color) }}
            >
              {!event.allDay && (
                <span className="text-muted-foreground mr-1">
                  {clockTime(event.start)}
                </span>
              )}
              {event.title}
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
