import type { CSSProperties } from "react";
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
    <div
      className="week-scroller flex-1 overflow-auto overscroll-contain relative"
      ref={timelineScrollRef}
    >
      <div className="week-canvas min-w-212.5 relative">
        <div className="week-heading grid grid-cols-[62px_repeat(7,_minmax(0,_1fr))] sticky top-0 bg-background z-[4]">
          <div className="text-[10px] text-muted-foreground pt-3 pl-3">GMT+7</div>
          {visibleDates.map((date) => (
            <div
              className="day-heading text-center h-10 text-muted-foreground p-[10px_0]"
              key={date}
            >
              {dayLabel(date).split(" ")[0]}{" "}
              <span
                className={`p-1.5 ${date === today ? "today bg-[#fa5353]! text-[white]! rounded-[6px]" : ""}`}
              >
                {Number(date.slice(-2))}
              </span>
            </div>
          ))}
          <div className="all-day min-h-7.5 [border-bottom:1px_solid_var(--calendar-secondary)] text-[10px] text-muted-foreground pl-3 pt-2">
            All-day
          </div>
          {visibleDates.map((date) => (
            <div
              className="all-day min-h-7.5 [border-bottom:1px_solid_var(--calendar-secondary)]"
              key={date}
            >
              {eventsOnDay(events, date)
                .filter((event) => event.allDay)
                .map((event) => (
                  <button
                    key={event.eventId}
                    className={`
                      month-event block w-full text-left p-[3px_5px] border-l-[4px] border-solid rounded-[4px] mt-[3px]
                      whitespace-nowrap overflow-hidden text-ellipsis text-[11px] border-[var(--calendar-border-color)]
                    `}
                    style={
                      {
                        "--calendar-border-color": eventColor(event.color),
                      } as CSSProperties
                    }
                    onClick={() => openEvent(event)}
                  >
                    {event.title}
                  </button>
                ))}
            </div>
          ))}
        </div>
        <div className="time-grid grid grid-cols-[62px_repeat(7,_minmax(0,_1fr))]">
          <div className="time-axis text-muted-foreground text-[10px] text-right pr-2">
            {Array.from({ length: 24 }, (_, hour) => (
              <div className="hour-label h-15 relative top-[-7px]" key={hour}>
                {String(hour).padStart(2, "0")}:00
              </div>
            ))}
          </div>
          {visibleDates.map((date) => (
            <div
              key={date}
              className={`
                day-column h-360 relative [border-left:1px_solid_var(--calendar-surface)]
                [background:repeating-linear-gradient(_to_bottom,_transparent_0px,_transparent_59px,_var(--calendar-grid)_59px,_var(--calendar-grid)_60px_)]
              `}
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
                  className={`
                    event-card flex flex-col items-start justify-start absolute border-l-[4px] border-solid rounded-[5px]
                    p-[5px_7px] overflow-hidden text-left text-[11px] leading-[1.35] [&_strong]:block
                    [&_strong]:font-semibold top-[var(--calendar-top)] h-[var(--calendar-height)]
                    left-[var(--calendar-left)] w-[var(--calendar-width)] border-[var(--calendar-border-color)]
                    bg-[var(--calendar-background)]
                  `}
                  draggable={!placement.event.readOnly && !calendar.save.isPending}
                  onDragStart={(event) =>
                    event.dataTransfer.setData("text/plain", placement.event.eventId)
                  }
                  onClick={() => openEvent(placement.event as EventRecord)}
                  style={
                    {
                      "--calendar-top": `${placement.top}px`,
                      "--calendar-height": `${placement.height}px`,
                      "--calendar-left": `${placement.left}%`,
                      "--calendar-width": `calc(${placement.width}% - 5px)`,
                      "--calendar-border-color": eventColor(placement.event.color),
                      "--calendar-background": `color-mix(in srgb, ${eventColor(placement.event.color)} 24%, var(--calendar-surface))`,
                    } as CSSProperties
                  }
                >
                  <strong>{placement.event.title}</strong>
                  <span className="opacity-70">
                    {clockTime(placement.event.start)}–{clockTime(placement.event.end)}
                  </span>
                  {!placement.event.readOnly && (
                    <span
                      className="resize-handle h-2 absolute bottom-0 left-0 right-0 cursor-ns-resize"
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
