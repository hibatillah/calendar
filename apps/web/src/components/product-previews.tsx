import {
  ArrowUpRight,
  CalendarDays,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  Menu,
  PanelLeft,
  Plus,
  Search,
  SquarePen,
} from "lucide-react";
import { useEffect, useState } from "react";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";

export function useInteractiveReady() {
  const [ready, setReady] = useState(false);

  // oxlint-disable-next-line react/set-state-in-effect -- Enable interactive previews only after hydration.
  useEffect(() => setReady(true), []);

  return ready;
}

export const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export const sampleEvents = [
  {
    title: "A little planning",
    day: 0,
    start: 9,
    duration: 1,
    color: "#7198bc",
    time: "09:00 – 10:00",
  },
  {
    title: "Make something good",
    day: 1,
    start: 10,
    duration: 2,
    color: "#d28b77",
    time: "10:00 – 12:00",
  },
  {
    title: "Coffee with Maya",
    day: 2,
    start: 9.5,
    duration: 1,
    color: "#bba16b",
    time: "09:30 – 10:30",
  },
  {
    title: "Room to focus",
    day: 3,
    start: 9,
    duration: 2.5,
    color: "#7b9f87",
    time: "09:00 – 11:30",
  },
  {
    title: "Design catch-up",
    day: 4,
    start: 10.5,
    duration: 1,
    color: "#7198bc",
    time: "10:30 – 11:30",
  },
  {
    title: "An afternoon outside",
    day: 5,
    start: 12,
    duration: 2,
    color: "#bba16b",
    time: "12:00 – 14:00",
  },
  {
    title: "Lunch, unhurried",
    day: 2,
    start: 12,
    duration: 1,
    color: "#d28b77",
    time: "12:00 – 13:00",
  },
];

export function MiniMonth({ mobile = false }: { mobile?: boolean }) {
  return (
    <div
      className={mobile ? "preview-mini-month mobile-month" : "preview-mini-month"}
      aria-hidden="true"
    >
      {weekdays.map((day) => (
        <span key={day} className="preview-weekday">
          {day.slice(0, 2)}
        </span>
      ))}
      {Array.from({ length: 35 }, (_, index) => {
        const date = index - 2;

        return (
          <span
            key={index}
            className={`${date < 1 || date > 31 ? "muted-date" : ""} ${date === 14 ? "preview-today" : ""} ${date >= 12 && date <= 18 ? "preview-selected-week" : ""}`}
          >
            <b>{date < 1 ? date + 30 : date > 31 ? date - 31 : date}</b>
            {mobile && date > 0 && date < 32 && (
              <i className="month-event-lines">
                {Array.from({ length: date % 4 }, (_, eventIndex) => (
                  <i
                    key={eventIndex}
                    style={{
                      background: ["#cc8e7c", "#7da289", "#7f9cb7"][eventIndex],
                    }}
                  />
                ))}
              </i>
            )}
          </span>
        );
      })}
    </div>
  );
}

export function ProductPreview() {
  const interactiveReady = useInteractiveReady();
  const [view, setView] = useState<"week" | "month">("week");

  return (
    <div
      className="product-preview"
      data-theme="light"
      aria-label="Interactive Calendar product preview with example events"
    >
      <aside className="preview-sidebar">
        <div className="preview-sidebar-tools">
          <ChevronDown size={16} />
          <PanelLeft size={16} />
          <span />
          <Search size={16} />
          <SquarePen size={16} />
        </div>
        <div className="preview-mini-heading">
          October 2026 <span>⌃ &nbsp;⌄</span>
        </div>
        <MiniMonth />
        <p className="preview-account">alex@example.com</p>
        {["Personal", "Work", "A little time outside"].map((calendar, index) => (
          <div className="preview-calendar" key={calendar}>
            <i style={{ background: ["#d38b77", "#7198bc", "#7b9f87"][index] }} />
            {calendar}
          </div>
        ))}
        <div className="preview-sidebar-bottom">
          <CircleHelp size={15} /> A little room for your time.
        </div>
      </aside>
      <div className="preview-main">
        <div className="preview-toolbar">
          <span className="preview-avatar">a</span>
          <span />
          <div className="preview-view-switch" aria-label="Preview calendar view">
            <button
              aria-pressed={view === "week"}
              disabled={!interactiveReady}
              onClick={() => setView("week")}
            >
              Week
            </button>
            <button
              aria-pressed={view === "month"}
              disabled={!interactiveReady}
              onClick={() => setView("month")}
            >
              Month
            </button>
          </div>
          <span className="preview-today-control">Today</span>
          <ChevronLeft size={17} />
          <ChevronRight size={17} />
        </div>
        <h2>October 2026</h2>
        {view === "week" ? (
          <>
            <div className="preview-day-headings">
              <span>GMT+7</span>
              {weekdays.map((day, index) => (
                <span key={day}>
                  {day}{" "}
                  <b className={index === 2 ? "preview-date-active" : ""}>
                    {12 + index}
                  </b>
                </span>
              ))}
            </div>
            <div className="preview-time-grid">
              <div className="preview-hours">
                {[9, 10, 11, 12, 13, 14].map((hour) => (
                  <span key={hour}>{String(hour).padStart(2, "0")}:00</span>
                ))}
              </div>
              <div className="preview-event-grid">
                {weekdays.map((day) => (
                  <div className="preview-column" key={day} />
                ))}
                {sampleEvents.map((event) => (
                  <Popover key={event.title}>
                    <PopoverTrigger asChild>
                      <button
                        className="preview-event"
                        disabled={!interactiveReady}
                        style={{
                          left: `calc(${(event.day * 100) / 7}% + 3px)`,
                          top: `${(event.start - 9) * 60}px`,
                          height: event.duration * 60 - 4,
                          background: `${event.color}40`,
                          borderColor: event.color,
                        }}
                      >
                        <strong>{event.title}</strong>
                        <span>{event.time}</span>
                      </button>
                    </PopoverTrigger>
                    <PopoverContent
                      className="preview-event-details"
                      data-theme="light"
                      side="top"
                    >
                      <span className="demo-label">EXAMPLE EVENT</span>
                      <h3>{event.title}</h3>
                      <p>
                        October {12 + event.day}, 2026 · {event.time}
                      </p>
                      <p>
                        This is a product preview. Your real calendar lives in the app.
                      </p>
                      <a href="/app">
                        Open calendar <ArrowUpRight size={14} />
                      </a>
                    </PopoverContent>
                  </Popover>
                ))}
                <div className="preview-now-line" style={{ top: 182 }}>
                  <i />
                </div>
              </div>
            </div>
          </>
        ) : (
          <div className="preview-large-month" aria-label="Month preview">
            {weekdays.map((day) => (
              <strong key={day}>{day}</strong>
            ))}
            {Array.from({ length: 35 }, (_, index) => (
              <div key={index}>
                <span>{index < 3 ? 28 + index : index === 34 ? 1 : index - 2}</span>
                {sampleEvents
                  .filter((event) => event.day + 14 === index)
                  .map((event) => (
                    <p key={event.title} style={{ borderColor: event.color }}>
                      {event.title}
                    </p>
                  ))}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export function PhonePreview() {
  return (
    <div
      className="phone-preview"
      data-theme="light"
      aria-label="Calendar mobile example: month above a daily agenda"
    >
      <div className="phone-status">
        <b>9:41</b>
        <span className="phone-island" />
        <span>▮▮▮ ▰</span>
      </div>
      <div className="phone-heading">
        <Menu size={19} />
        <strong>October</strong>
        <CalendarDays size={19} />
      </div>
      <MiniMonth mobile />
      <div className="phone-agenda-date">
        Wednesday, October 14 <span>3 events</span>
      </div>
      {[
        sampleEvents[2],
        sampleEvents[6],
        {
          ...sampleEvents[0],
          title: "A moment to reset",
          time: "16:00 – 16:30",
          color: "#7b9f87",
        },
      ].map((event) => (
        <div className="phone-agenda-event" key={event.title}>
          <span>{event.time.slice(0, 5)}</span>
          <i style={{ background: event.color }} />
          <div>
            <strong>{event.title}</strong>
            <p>{event.time}</p>
          </div>
        </div>
      ))}
      <span className="phone-add">
        <Plus size={25} />
      </span>
      <div className="phone-home-indicator" />
    </div>
  );
}
