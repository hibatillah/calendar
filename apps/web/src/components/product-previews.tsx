import type { CSSProperties } from "react";
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
    color: "#419ce3",
    time: "09:00 – 10:00",
  },
  {
    title: "Make something good",
    day: 1,
    start: 10,
    duration: 2,
    color: "#ef795f",
    time: "10:00 – 12:00",
  },
  {
    title: "Coffee with Maya",
    day: 2,
    start: 9.5,
    duration: 1,
    color: "#d6a025",
    time: "09:30 – 10:30",
  },
  {
    title: "Room to focus",
    day: 3,
    start: 9,
    duration: 2.5,
    color: "#41ad78",
    time: "09:00 – 11:30",
  },
  {
    title: "Design catch-up",
    day: 4,
    start: 10.5,
    duration: 1,
    color: "#419ce3",
    time: "10:30 – 11:30",
  },
  {
    title: "An afternoon outside",
    day: 5,
    start: 12,
    duration: 2,
    color: "#d6a025",
    time: "12:00 – 14:00",
  },
  {
    title: "Lunch, unhurried",
    day: 2,
    start: 12,
    duration: 1,
    color: "#ef795f",
    time: "12:00 – 13:00",
  },
];

export function MiniMonth({ mobile = false }: { mobile?: boolean }) {
  return (
    <div
      className={
        mobile
          ? "preview-mini-month grid grid-cols-7 text-[10px] gap-[2px_0] [&_>_span]:text-center [&_>_span]:p-[5px_0] [&_>_span]:min-w-0 [&_b]:font-medium [&_b]:relative [&_b]:z-[1] [.hero-product_&_b]:text-[#343b38] mobile-month gap-[5px_2px] [&_>_span]:h-9 [&_>_span]:text-[12px] [&_>_span]:p-0!"
          : "preview-mini-month grid grid-cols-7 text-[10px] gap-[2px_0] [&_>_span]:text-center [&_>_span]:p-[5px_0] [&_>_span]:min-w-0 [&_b]:font-medium [&_b]:relative [&_b]:z-[1] [.hero-product_&_b]:text-[#343b38]"
      }
      aria-hidden="true"
    >
      {weekdays.map((day) => (
        <span
          key={day}
          className="preview-weekday text-muted-foreground [.mobile-month_&]:h-5.5 [.mobile-month_&]:text-[9px]"
        >
          {day.slice(0, 2)}
        </span>
      ))}
      {Array.from({ length: 35 }, (_, index) => {
        const date = index - 2;

        return (
          <span
            key={index}
            className={`${date < 1 || date > 31 ? "muted-date [.hero-product_.preview-mini-month_&]:opacity-[0.55] opacity-[0.36]" : ""} ${date === 14 ? "preview-today [.hero-product_.preview-mini-month_&_b]:text-[#fff] relative [&::before]:absolute [&::before]:content-[''] [&::before]:top-0.5 [&::before]:bottom-0.5 [&::before]:left-0.5 [&::before]:right-0.5 [&::before]:bg-[#eb6864] [&::before]:rounded-[5px] [.mobile-month_&::before]:bottom-[auto] [.mobile-month_&::before]:h-5 [.mobile-month_&::before]:left-[5px] [.mobile-month_&::before]:right-[5px] [.mobile-month_&::before]:top-0" : ""} ${date >= 12 && date <= 18 ? "preview-selected-week bg-selected [.mobile-month_&]:bg-transparent" : ""}`}
          >
            <b>{date < 1 ? date + 30 : date > 31 ? date - 31 : date}</b>
            {mobile && date > 0 && date < 32 && (
              <i className="month-event-lines grid gap-0.5 m-[3px_3px_0] [&_>_i]:h-0.5 [&_>_i]:rounded-[3px]">
                {Array.from({ length: date % 4 }, (_, eventIndex) => (
                  <i
                    key={eventIndex}
                    className="bg-[var(--calendar-background)]"
                    style={
                      {
                        "--calendar-background": ["#cc8e7c", "#7da289", "#7f9cb7"][
                          eventIndex
                        ],
                      } as CSSProperties
                    }
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
      className={`
        product-preview h-162.5 flex text-left text-foreground [border:1px_solid_var(--calendar-border)]
        bg-background rounded-[12px] overflow-hidden shadow-[-24px_28px_32px_-16px_#2f594738] text-[11px]
        max-[700px]:w-187.5 max-[700px]:h-162.5 max-[700px]:[transform:scale(0.66)]
        max-[700px]:origin-top-left
      `}
      data-theme="light"
      aria-label="Interactive Calendar product preview with example events"
    >
      <aside className="preview-sidebar w-49.5 bg-sidebar shrink-0 p-[15px_13px] flex flex-col text-muted-foreground max-[1000px]:w-[175px] max-[700px]:w-40">
        <div className="preview-sidebar-tools flex gap-3.5 items-center text-muted-foreground [&_>_span]:flex-1">
          <ChevronDown size={16} />
          <PanelLeft size={16} />
          <span />
          <Search size={16} />
          <SquarePen size={16} />
        </div>
        <div className="preview-mini-heading flex justify-between text-[10px] m-[26px_1px_15px] [&_span]:text-muted-foreground">
          October 2026 <span>⌃ &nbsp;⌄</span>
        </div>
        <MiniMonth />
        <p className="preview-account m-[29px_2px_9px]! text-muted-foreground text-[10px]">
          alex@example.com
        </p>
        {["Personal", "Work", "A little time outside"].map((calendar, index) => (
          <div
            className="preview-calendar flex items-center gap-[9px] m-[3px_2px] text-[11px] [&_i]:w-2.5 [&_i]:h-2.5 [&_i]:rounded-[3px]"
            key={calendar}
          >
            <i
              className="bg-[var(--calendar-background)]"
              style={
                {
                  "--calendar-background": ["#d38b77", "#419ce3", "#41ad78"][index],
                } as CSSProperties
              }
            />
            {calendar}
          </div>
        ))}
        <div className="preview-sidebar-bottom mt-auto flex items-center gap-[7px] text-[9px] text-muted-foreground">
          <CircleHelp size={15} /> A little room for your time.
        </div>
      </aside>
      <div
        className={`
          preview-main min-w-0 flex-1 [.marketing-page_&_h2]:text-[23px]
          [.marketing-page_&_h2]:tracking-[-0.8px] [.marketing-page_&_h2]:p-[7px_18px_19px]
          [.marketing-page_&_h2]:font-semibold
        `}
      >
        <div className="preview-toolbar flex items-center gap-[15px] h-11.5 p-[8px_17px] text-muted-foreground [&_>_span:nth-child(2)]:flex-1">
          <span className="preview-avatar w-[21px] h-[21px] rounded-full [background:linear-gradient(150deg,_#7496ad,_#c2bfa7)] text-[#243746] text-[12px] text-center">
            a
          </span>
          <span />
          <div
            className={`
              preview-view-switch flex p-[3px] bg-sidebar border-0 rounded-full [&_button]:bg-transparent
              [&_button]:text-muted-foreground [&_button]:border-0 [&_button]:p-[3px_9px] [&_button]:rounded-full
              [&_button]:text-[10px] [&_button[aria-pressed=true]]:bg-selected
              [&_button[aria-pressed=true]]:text-foreground
            `}
            aria-label="Preview calendar view"
          >
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
          <span className="preview-today-control text-[10px]">Today</span>
          <ChevronLeft size={17} />
          <ChevronRight size={17} />
        </div>
        <h2>October 2026</h2>
        {view === "week" ? (
          <>
            <div
              className={`
                preview-day-headings grid grid-cols-[45px_repeat(7,_1fr)] text-muted-foreground text-[10px]
                [border-bottom:1px_solid_var(--calendar-grid)] pb-2 text-center [&_>_span:first-child]:text-[8px]
                [&_>_span:first-child]:pt-1 [&_b]:ml-[3px] [&_b]:font-medium
              `}
            >
              <span>GMT+7</span>
              {weekdays.map((day, index) => (
                <span key={day}>
                  {day}{" "}
                  <b
                    className={
                      index === 2
                        ? "preview-date-active text-[white] bg-[#ea6764] p-[3px] rounded-[4px]"
                        : ""
                    }
                  >
                    {12 + index}
                  </b>
                </span>
              ))}
            </div>
            <div className="preview-time-grid flex">
              <div className="preview-hours w-[45px] shrink-0 flex flex-col text-muted-foreground text-[9px] text-center [&_span]:h-15 [&_span]:pt-1">
                {[9, 10, 11, 12, 13, 14].map((hour) => (
                  <span key={hour}>{String(hour).padStart(2, "0")}:00</span>
                ))}
              </div>
              <div
                className={`
                  preview-event-grid flex relative flex-1 h-135
                  [background:repeating-linear-gradient(_to_bottom,_transparent_0,_transparent_59px,_var(--calendar-grid)_59px,_var(--calendar-grid)_60px_)]
                `}
              >
                {weekdays.map((day) => (
                  <div
                    className="preview-column w-[calc(100%_/_7)] [border-left:1px_solid_var(--calendar-grid)]"
                    key={day}
                  />
                ))}
                {sampleEvents.map((event) => (
                  <Popover key={event.title}>
                    <PopoverTrigger asChild>
                      <button
                        className={`
                          preview-event absolute w-[calc(100%_/_7_-_6px)] border-0 border-l-[3px] border-solid rounded-[5px] p-1.5
                          text-left text-foreground overflow-hidden text-[10px] leading-[1.3] [transition:filter_0.15s]
                          [&:hover]:[filter:brightness(1.3)] [&_strong]:font-medium [&_strong]:block [&_>_span]:opacity-[0.7]
                          [&_>_span]:text-[8px] [&_>_span]:block [&_>_span]:mt-[3px] left-[var(--calendar-left)]
                          top-[var(--calendar-top)] h-[var(--calendar-height)] bg-[var(--calendar-background)]
                          border-[var(--calendar-border-color)]
                        `}
                        disabled={!interactiveReady}
                        style={
                          {
                            "--calendar-left": `calc(${(event.day * 100) / 7}% + 3px)`,
                            "--calendar-top": `${(event.start - 9) * 60}px`,
                            "--calendar-height": `${event.duration * 60 - 4}px`,
                            "--calendar-background": `${event.color}66`,
                            "--calendar-border-color": event.color,
                          } as CSSProperties
                        }
                      >
                        <strong>{event.title}</strong>
                        <span>{event.time}</span>
                      </button>
                    </PopoverTrigger>
                    <PopoverContent
                      className={`
                        preview-event-details p-5 [font-family:Inter,_sans-serif] bg-grid text-foreground
                        border-[var(--calendar-border)] [&_h3]:text-[16px] [&_h3]:mt-2.5 [&_p]:text-muted-foreground
                        [&_p]:m-[10px_0] [&_p]:text-[12px] [&_p]:leading-[1.6] [&_a]:flex [&_a]:gap-[5px] [&_a]:items-center
                        [&_a]:text-[12px]
                      `}
                      data-theme="light"
                      side="top"
                    >
                      <span className="demo-label [.preview-event-details_&]:text-[9px] [.preview-event-details_&]:tracking-[1px] [.preview-event-details_&]:text-muted-foreground">
                        EXAMPLE EVENT
                      </span>
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
                <div
                  className={`
                    preview-now-line absolute w-full h-[1px] bg-[#bd5c57] pointer-events-none [&_i]:absolute
                    [&_i]:left-[-3px] [&_i]:top-[-2px] [&_i]:w-[5px] [&_i]:h-[5px] [&_i]:rounded-full [&_i]:bg-[#d36c64]
                    top-[182px]
                  `}
                >
                  <i />
                </div>
              </div>
            </div>
          </>
        ) : (
          <div
            className={`
              preview-large-month grid grid-cols-7 text-[10px] text-muted-foreground [&_>_strong]:font-medium
              [&_>_strong]:text-center [&_>_strong]:pb-[9px] [&_>_div]:h-[69px] [&_>_div]:p-1
              [&_>_div]:[border-left:1px_solid_var(--calendar-grid)]
              [&_>_div]:[border-top:1px_solid_var(--calendar-grid)] [&_>_div]:overflow-hidden
              [&_>_div_>_span]:block [&_>_div_>_span]:text-right [&_p]:text-[8px] [&_p]:border-l-[3px] border-solid
              [&_p]:pl-[3px] [&_p]:text-foreground [&_p]:mt-[5px] [&_p]:whitespace-nowrap [&_p]:overflow-hidden
              [&_p]:text-ellipsis
            `}
            aria-label="Month preview"
          >
            {weekdays.map((day) => (
              <strong key={day}>{day}</strong>
            ))}
            {Array.from({ length: 35 }, (_, index) => (
              <div key={index}>
                <span>{index < 3 ? 28 + index : index === 34 ? 1 : index - 2}</span>
                {sampleEvents
                  .filter((event) => event.day + 14 === index)
                  .map((event) => (
                    <p
                      key={event.title}
                      className="border-[var(--calendar-border-color)]"
                      style={
                        { "--calendar-border-color": event.color } as CSSProperties
                      }
                    >
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
      className={`
        phone-preview z-[1] relative w-71.5 h-145 bg-background text-foreground [border:6px_solid_#353732]
        rounded-[39px] shadow-[0_20px_35px_#41473640] p-[10px_15px] shrink-0 max-[700px]:[transform:none] z-0
      `}
      data-theme="light"
      aria-label="Calendar mobile example: month above a daily agenda"
    >
      <div className="phone-status flex justify-between text-[9px] items-center p-[0_6px]">
        <b>9:41</b>
        <span className="phone-island h-[19px] w-17.5 bg-[#0a0a09] rounded-full" />
        <span>▮▮▮ ▰</span>
      </div>
      <div className="phone-heading flex justify-between items-center m-[23px_0_20px] [&_strong]:text-[17px] [&_strong]:font-medium">
        <Menu size={19} />
        <strong>October</strong>
        <CalendarDays size={19} />
      </div>
      <MiniMonth mobile />
      <div className="phone-agenda-date flex justify-between [border-top:1px_solid_var(--calendar-grid)] p-[16px_0] mt-2 text-[10px] [&_>_span]:text-muted-foreground [&_>_span]:text-[9px]">
        Wednesday, October 14 <span>3 events</span>
      </div>
      {[
        sampleEvents[2],
        sampleEvents[6],
        {
          ...sampleEvents[0],
          title: "A moment to reset",
          time: "16:00 – 16:30",
          color: "#41ad78",
        },
      ].map((event) => (
        <div
          className={`
            phone-agenda-event flex gap-[9px] items-stretch p-[12px_0]
            [border-bottom:1px_solid_var(--calendar-grid)] text-[10px] [&_>_span]:text-muted-foreground
            [&_>_span]:text-[9px] [&_>_span]:w-7.5 [&_>_i]:w-[3px] [&_>_i]:rounded-[3px] [&_strong]:font-medium
            [&_p]:text-muted-foreground [&_p]:text-[8px] [&_p]:mt-0
          `}
          key={event.title}
        >
          <span>{event.time.slice(0, 5)}</span>
          <i
            className="bg-[var(--calendar-background)]"
            style={{ "--calendar-background": event.color } as CSSProperties}
          />
          <div>
            <strong>{event.title}</strong>
            <p>{event.time}</p>
          </div>
        </div>
      ))}
      <span className="phone-add absolute right-4.5 bottom-7.5 bg-secondary text-foreground rounded-full h-10.5 w-10.5 grid place-items-center">
        <Plus size={25} />
      </span>
      <div className="phone-home-indicator absolute bottom-[7px] w-22.5 h-[3px] rounded-[5px] bg-foreground left-[calc(50%_-_45px)]" />
    </div>
  );
}
