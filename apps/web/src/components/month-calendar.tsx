import type { CSSProperties } from "react";
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
      className="month-grid grid grid-cols-[repeat(7,_minmax(0,_1fr))] flex-1 min-h-0 overflow-auto grid-rows-[var(--calendar-grid-template-rows)]"
      style={
        {
          "--calendar-grid-template-rows": `repeat(${monthDates.length / 7}, minmax(110px,1fr))`,
        } as CSSProperties
      }
    >
      {monthDates.map((date) => (
        <div
          className="month-cell [border-top:1px_solid_var(--calendar-grid)] [border-right:1px_solid_var(--calendar-grid)] min-h-27.5 p-[8px_5px] overflow-hidden"
          key={date}
          onDoubleClick={() => newEvent(date)}
        >
          <button
            className={`block ml-auto px-1.5 py-1 mb-1 ${date === today ? "today bg-[#fa5353]! text-[white]! rounded-[6px]" : ""} ${date.slice(0, 7) !== selectedDate.slice(0, 7) ? "muted text-muted-foreground" : ""}`}
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
              className={`
                month-event block w-full text-left p-[3px_5px] border-l-[4px] border-solid rounded-[4px] mt-[3px]
                whitespace-nowrap overflow-hidden text-ellipsis text-[11px] border-[var(--calendar-border-color)]
              `}
              style={
                { "--calendar-border-color": eventColor(event.color) } as CSSProperties
              }
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
