import { addMonths, monthLabel } from "@calendar/calendar";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "./ui/button";
import type { CalendarController } from "./use-calendar-controller";

const weekdayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

export function MiniCalendar({ state }: { state: CalendarController }) {
  const {
    selectedDate,
    setSelectedDate,
    setFirstDay,
    weekStart,
    monthDates,
    visibleDates,
    today,
  } = state;

  return (
    <>
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {monthLabel(selectedDate)}
        </span>
        <div className="flex">
          <Button
            variant="ghost"
            size="icon"
            aria-label="Previous sidebar month"
            onClick={() => setSelectedDate(addMonths(selectedDate, -1))}
          >
            <ChevronLeft size={17} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Next sidebar month"
            onClick={() => setSelectedDate(addMonths(selectedDate, 1))}
          >
            <ChevronRight size={17} />
          </Button>
        </div>
      </div>
      <div className="mini-grid">
        {Array.from({ length: 7 }, (_, index) => (
          <span
            key={index}
            className="text-center text-[11px] text-muted-foreground py-2"
          >
            {weekdayNames[(index + weekStart) % 7]}
          </span>
        ))}
        {monthDates.map((date) => (
          <button
            key={date}
            aria-label={date}
            aria-pressed={visibleDates.includes(date)}
            className={`${visibleDates.includes(date) ? "selected" : ""} ${date === today ? "today" : ""} ${date.slice(0, 7) !== selectedDate.slice(0, 7) ? "muted" : ""}`}
            onClick={() => {
              setSelectedDate(date);
              setFirstDay(date);
            }}
          >
            {Number(date.slice(-2))}
          </button>
        ))}
      </div>
    </>
  );
}
