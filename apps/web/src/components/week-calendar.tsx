import { type EventRecord } from "@calendar/api";
import {
  clockTime,
  dayLabel,
  eventColor,
  eventsOnDay,
  localInstant,
  timedPlacement,
} from "@calendar/calendar";
import type { CalendarController } from "./use-calendar-controller";

export function WeekCalendar({ state }: { state: CalendarController }) {
  const {
    today,
    calendar,
    events,
    visibleDates,
    timelineScrollRef,
    newEvent,
    saveEvent,
    openEvent,
  } = state;

  return (
    <div className="week-scroller" ref={timelineScrollRef}>
      <div className="week-canvas">
        <div className="week-heading">
          <div className="text-[10px] text-muted-foreground pt-3 pl-3">GMT+7</div>
          {visibleDates.map((date) => (
            <div className="day-heading" key={date}>
              {dayLabel(date).split(" ")[0]}{" "}
              <span className={`p-1.5 ${date === today ? "today" : ""}`}>
                {Number(date.slice(-2))}
              </span>
            </div>
          ))}
          <div className="all-day text-[10px] text-muted-foreground pl-3 pt-2">
            All-day
          </div>
          {visibleDates.map((date) => (
            <div className="all-day" key={date}>
              {eventsOnDay(events, date)
                .filter((event) => event.allDay)
                .map((event) => (
                  <button
                    key={event.eventId}
                    className="month-event"
                    style={{ borderColor: eventColor(event.color) }}
                    onClick={() => openEvent(event)}
                  >
                    {event.title}
                  </button>
                ))}
            </div>
          ))}
        </div>
        <div className="time-grid">
          <div className="time-axis">
            {Array.from({ length: 24 }, (_, hour) => (
              <div className="hour-label" key={hour}>
                {String(hour).padStart(2, "0")}:00
              </div>
            ))}
          </div>
          {visibleDates.map((date) => (
            <div
              key={date}
              className="day-column"
              onDoubleClick={(event) => {
                if (event.target !== event.currentTarget) return;

                const minutes = Math.min(
                  1425,
                  Math.floor(
                    (event.clientY - event.currentTarget.getBoundingClientRect().top) /
                      15,
                  ) * 15,
                );

                newEvent(
                  date,
                  `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`,
                );
              }}
              onDragOver={(event) => event.preventDefault()}
              onDrop={(event) => {
                event.preventDefault();

                const moved = events.find(
                  (item) => item.eventId === event.dataTransfer.getData("text/plain"),
                );

                if (!moved || moved.readOnly) return;

                const minutes = Math.min(
                  1425,
                  Math.max(
                    0,
                    Math.round(
                      (event.clientY -
                        event.currentTarget.getBoundingClientRect().top) /
                        15,
                    ) * 15,
                  ),
                );
                const start = localInstant(
                  date,
                  `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`,
                );
                const input = moved;

                void saveEvent(
                  {
                    ...input,
                    start,
                    end: new Date(
                      Date.parse(start) +
                        Date.parse(moved.end) -
                        Date.parse(moved.start),
                    ).toISOString(),
                  },
                  moved.revision,
                );
              }}
            >
              {timedPlacement(events, date).map((placement) => (
                <button
                  key={placement.event.eventId}
                  className="event-card"
                  draggable={!placement.event.readOnly && !calendar.save.isPending}
                  onDragStart={(event) =>
                    event.dataTransfer.setData("text/plain", placement.event.eventId)
                  }
                  onClick={() => openEvent(placement.event as EventRecord)}
                  style={{
                    top: placement.top,
                    height: placement.height,
                    left: `${placement.left}%`,
                    width: `calc(${placement.width}% - 5px)`,
                    borderColor: eventColor(placement.event.color),
                    background: `color-mix(in srgb, ${eventColor(placement.event.color)} 24%, var(--calendar-surface))`,
                  }}
                >
                  <strong>{placement.event.title}</strong>
                  <span className="opacity-70">
                    {clockTime(placement.event.start)}–{clockTime(placement.event.end)}
                  </span>
                  {!placement.event.readOnly && (
                    <span
                      className="resize-handle"
                      onPointerDown={(pointer) => {
                        pointer.preventDefault();
                        pointer.stopPropagation();

                        const origin = pointer.clientY;
                        const event = placement.event as EventRecord;
                        const release = (end: PointerEvent) => {
                          document.removeEventListener("pointerup", release);

                          const difference =
                            Math.round((end.clientY - origin) / 15) * 15;

                          if (!difference) return;

                          const input = event;

                          void saveEvent(
                            {
                              ...input,
                              end: new Date(
                                Math.max(
                                  Date.parse(event.start) + 900000,
                                  Date.parse(event.end) + difference * 60000,
                                ),
                              ).toISOString(),
                            },
                            event.revision,
                          );
                        };

                        document.addEventListener("pointerup", release, {
                          once: true,
                        });
                      }}
                    />
                  )}
                </button>
              ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
