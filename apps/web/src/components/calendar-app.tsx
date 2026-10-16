import { useEffect, useMemo, useRef, useState } from "react";
import { useCalendarTheme, type CalendarTheme } from "./use-calendar-theme";
import {
  CalendarDays,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  CircleHelp,
  PanelLeft,
  Plus,
  Settings,
  SquarePen,
  X,
} from "lucide-react";
import {
  addDays,
  addMonths,
  clockTime,
  dateKey,
  dayLabel,
  DISPLAY_ZONE,
  eventColor,
  eventsOnDay,
  localInstant,
  monthDays,
  monthLabel,
  startOfWeek,
  timedPlacement,
  type WeekStart,
} from "@calendar/calendar";
import {
  ConnectCalendarAccountDocument,
  createTransport,
  DisconnectCalendarAccountDocument,
  type EventInput,
  type EventRecord,
} from "@calendar/api";
import { useCalendar, type SnapshotStorage } from "@calendar/api/react";
import { useQueryClient } from "@tanstack/react-query";
import { Button } from "./ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "./ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu";
const transport = createTransport("/api/graphql");
const storage: SnapshotStorage = {
  async read(key) {
    const value = localStorage.getItem(`calendar:snapshot:${key}`);
    return value ? JSON.parse(value) : undefined;
  },
  async write(key, value) {
    localStorage.setItem(`calendar:snapshot:${key}`, JSON.stringify(value));
  },
  async clear() {
    Object.keys(localStorage)
      .filter((key) => key.startsWith("calendar:snapshot:"))
      .forEach((key) => localStorage.removeItem(key));
  },
};
const weekdayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
export function CalendarApp() {
  const { theme, selectTheme } = useCalendarTheme();
  const [today, setToday] = useState(dateKey);
  const [selectedDate, setSelectedDate] = useState(today);
  const [firstDay, setFirstDay] = useState(() => startOfWeek(today));
  const [weekStart, setWeekStart] = useState<WeekStart>(1);
  const [view, setView] = useState<"week" | "month">("week");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<"general" | "accounts">(
    "general",
  );
  const [activityOpen, setActivityOpen] = useState(false);
  const [activity, setActivity] = useState<string[]>([]);
  const [hiddenCalendars, setHiddenCalendars] = useState<Set<string>>(
    new Set(),
  );
  const [draft, setDraft] = useState<EventInput | null>(null);
  const [saveCommand, setSaveCommand] = useState<{
    event: EventInput;
    operationId: string;
    expectedRevision: number;
  } | null>(null);
  const [message, setMessage] = useState("");
  const [accountBusy, setAccountBusy] = useState(false);
  const queryClient = useQueryClient();
  const monthDates = useMemo(
    () => monthDays(selectedDate, weekStart),
    [selectedDate, weekStart],
  );
  const rangeStart = view === "month" ? monthDates[0] : firstDay;
  const calendar = useCalendar(transport, rangeStart, storage);
  const accounts = calendar.data?.googleConnection.accounts ?? [];
  const calendars = accounts.flatMap((account) =>
    account.calendars.map((item) => ({
      ...item,
      writable:
        account.writeAccess && ["owner", "writer"].includes(item.accessRole),
    })),
  );
  const events = (calendar.data?.events ?? []).filter(
    (event) => !hiddenCalendars.has(event.calendarId),
  );
  const visibleDates =
    view === "month"
      ? monthDates
      : Array.from({ length: 7 }, (_, index) => addDays(firstDay, index));
  const scroller = useRef<HTMLDivElement>(null);
  const scrollAccumulator = useRef(0);
  const log = (action: string) =>
    setActivity((previous) =>
      [`${new Date().toLocaleTimeString()} · ${action}`, ...previous].slice(
        0,
        80,
      ),
    );
  useEffect(() => {
    const saved = localStorage.getItem("calendar:week-start");
    if (saved === "0" || saved === "1") {
      const start = Number(saved) as WeekStart;
      setWeekStart(start);
      setFirstDay(startOfWeek(dateKey(), start));
    }
    const interval = setInterval(() => setToday(dateKey()), 60_000);
    return () => clearInterval(interval);
  }, []);
  const goToday = () => {
    const current = dateKey();
    setSelectedDate(current);
    setFirstDay(startOfWeek(current, weekStart));
  };
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if (
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        draft ||
        settingsOpen ||
        (event.target instanceof HTMLElement &&
          (event.target.isContentEditable ||
            ["INPUT", "TEXTAREA", "SELECT"].includes(event.target.tagName)))
      )
        return;
      if (event.key.toLowerCase() === "w") setView("week");
      if (event.key.toLowerCase() === "m") setView("month");
      if (event.key.toLowerCase() === "t") goToday();
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [weekStart, draft, settingsOpen]);
  useEffect(() => {
    const element = scroller.current;
    if (!element) return;
    element.scrollTop = 7 * 60;
    const scroll = (event: WheelEvent) => {
      if (Math.abs(event.deltaX) <= Math.abs(event.deltaY) && !event.shiftKey)
        return;
      event.preventDefault();
      scrollAccumulator.current += event.shiftKey ? event.deltaY : event.deltaX;
      if (Math.abs(scrollAccumulator.current) > 80) {
        const direction = Math.sign(scrollAccumulator.current);
        setFirstDay((previous) => {
          const next = addDays(previous, direction);
          setSelectedDate(next);
          return next;
        });
        scrollAccumulator.current = 0;
      }
    };
    element.addEventListener("wheel", scroll, { passive: false });
    return () => element.removeEventListener("wheel", scroll);
  }, [view]);
  const navigate = (direction: number) => {
    if (view === "month")
      setSelectedDate((previous) => addMonths(previous, direction));
    else {
      const next = addDays(firstDay, direction * 7);
      setFirstDay(next);
      setSelectedDate(next);
    }
  };
  const newEvent = (date = selectedDate, time = "09:00") => {
    const selectedCalendar =
      calendars.find((item) => item.writable && item.primary) ??
      calendars.find((item) => item.writable);
    if (!selectedCalendar) {
      setSettingsTab("accounts");
      setSettingsOpen(true);
      setMessage(
        "Connect a Google account and approve calendar editing first.",
      );
      return;
    }
    const start = localInstant(date, time);
    setSaveCommand(null);
    setDraft({
      eventId: crypto.randomUUID(),
      calendarId: selectedCalendar.id,
      title: "",
      start,
      end: new Date(Date.parse(start) + 3600000).toISOString(),
      timezone: DISPLAY_ZONE,
      allDay: false,
      description: "",
      location: "",
      color:
        parseInt(selectedCalendar.backgroundColor.replace("#", ""), 16) ||
        5614047,
      revision: 0,
      readOnly: false,
    });
  };
  const saveEvent = async (
    event: EventInput,
    expectedRevision: number,
    fromEditor = false,
  ) => {
    if (calendar.save.isPending || calendar.isError) {
      setMessage("Wait for a working connection before saving.");
      return;
    }
    if (
      !event.title.trim() ||
      Date.parse(event.end) <= Date.parse(event.start)
    ) {
      setMessage("Enter a title and an end time after the start.");
      return;
    }
    const command =
      fromEditor &&
      saveCommand &&
      JSON.stringify(saveCommand.event) === JSON.stringify(event)
        ? saveCommand
        : { event, expectedRevision, operationId: crypto.randomUUID() };
    if (fromEditor) setSaveCommand(command);
    setMessage("Saving…");
    log("Saving event");
    try {
      await calendar.save.mutateAsync(command);
      if (fromEditor) {
        setDraft(null);
        setSaveCommand(null);
      }
      setMessage("Saved");
      log("Event saved");
    } catch (error) {
      setMessage(
        error instanceof Error
          ? error.message
          : "Save failed. Your draft is still available.",
      );
      log("Event save failed");
    }
  };
  const connectAccount = async (accountId?: string) => {
    const popup = window.open("about:blank", "_blank");
    setAccountBusy(true);
    try {
      const result = await transport(ConnectCalendarAccountDocument, {
        accountId,
      });
      if (popup) popup.location.href = result.connectGoogle;
      else window.location.href = result.connectGoogle;
      log("Google sign-in started");
    } catch (error) {
      popup?.close();
      setMessage((error as Error).message);
    } finally {
      setAccountBusy(false);
    }
  };
  const disconnectAccount = async (accountId: string) => {
    setAccountBusy(true);
    try {
      await transport(DisconnectCalendarAccountDocument, { accountId });
      await queryClient.cancelQueries({ queryKey: ["calendar"] });
      queryClient.removeQueries({ queryKey: ["calendar"] });
      await storage.clear();
      await calendar.refetch();
      log("Google account disconnected");
    } catch (error) {
      setMessage((error as Error).message);
    } finally {
      setAccountBusy(false);
    }
  };
  const openEvent = (event: EventRecord) => {
    setSaveCommand(null);
    const input = event;
    setDraft(input);
  };
  const renderMonthPicker = () => (
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
  return (
    <div className="calendar-shell">
      {sidebarOpen && (
        <aside className="sidebar">
          <div className="flex items-center justify-between">
            <div className="flex gap-1">
              <ChevronDown size={18} className="mt-2 text-muted-foreground" />
              <Button
                variant="ghost"
                size="icon"
                aria-label="Close sidebar"
                onClick={() => setSidebarOpen(false)}
              >
                <PanelLeft size={18} />
              </Button>
            </div>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Create event"
              onClick={() => newEvent()}
            >
              <SquarePen size={18} />
            </Button>
          </div>
          <div>{renderMonthPicker()}</div>
          <div className="flex-1 overflow-auto space-y-5">
            {accounts.map((account) => (
              <section key={account.id}>
                <p
                  className="text-xs text-muted-foreground mb-2 truncate"
                  title={account.email}
                >
                  {account.email}
                </p>
                {account.calendars.map((item) => (
                  <button
                    key={item.id}
                    className="flex w-full items-center gap-3 rounded-md px-2 py-2 text-left hover:bg-secondary/30"
                    onClick={() =>
                      setHiddenCalendars((previous) => {
                        const next = new Set(previous);
                        next.has(item.id)
                          ? next.delete(item.id)
                          : next.add(item.id);
                        return next;
                      })
                    }
                    aria-pressed={!hiddenCalendars.has(item.id)}
                  >
                    <span
                      className="h-3.5 w-3.5 shrink-0 rounded"
                      style={{
                        background: hiddenCalendars.has(item.id)
                          ? "transparent"
                          : item.backgroundColor,
                        border: `1px solid ${item.backgroundColor}`,
                      }}
                    />
                    <span className="truncate">{item.summary}</span>
                  </button>
                ))}
              </section>
            ))}
            {!accounts.length && (
              <div className="text-muted-foreground leading-6 px-2">
                Your calendars will appear here.
                <Button
                  variant="ghost"
                  onClick={() => {
                    setSettingsTab("accounts");
                    setSettingsOpen(true);
                  }}
                >
                  Connect Google
                </Button>
              </div>
            )}
          </div>
          <Popover open={activityOpen} onOpenChange={setActivityOpen}>
            <PopoverTrigger asChild>
              <Button
                variant="ghost"
                className="justify-start text-muted-foreground"
              >
                <CircleHelp size={17} />
                Activity
              </Button>
            </PopoverTrigger>
            <PopoverContent
              className="w-80 max-h-72 overflow-auto overscroll-contain"
              side="right"
            >
              <h3 className="font-semibold mb-3">Activity</h3>
              {activity.length ? (
                activity.map((entry, index) => (
                  <p
                    key={index}
                    className="text-xs text-muted-foreground py-2 border-b"
                  >
                    {entry}
                  </p>
                ))
              ) : (
                <p className="text-muted-foreground text-xs">
                  No activity this session.
                </p>
              )}
            </PopoverContent>
          </Popover>
        </aside>
      )}
      <main className="main-calendar">
        <header className="toolbar">
          <div className="flex items-center gap-3">
            {!sidebarOpen && (
              <Button
                variant="ghost"
                size="icon"
                aria-label="Open sidebar"
                onClick={() => setSidebarOpen(true)}
              >
                <PanelLeft size={18} />
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <button
                  aria-label="Account menu"
                  className="relative rounded-full w-8 h-8 bg-secondary overflow-hidden flex items-center justify-center"
                >
                  <CalendarDays size={18} />
                  {accounts[0]?.picture && (
                    <img
                      className="absolute inset-0 h-full w-full object-cover"
                      onError={(event) => {
                        event.currentTarget.style.display = "none";
                      }}
                      src={accounts[0].picture}
                      alt=""
                      referrerPolicy="no-referrer"
                    />
                  )}
                </button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="start" className="w-64">
                <div className="px-2 py-3">
                  <strong>{accounts[0]?.name || "Calendar"}</strong>
                  <p className="text-xs text-muted-foreground truncate">
                    {accounts[0]?.email || "Local preview"}
                  </p>
                </div>
                <DropdownMenuItem onSelect={() => setSettingsOpen(true)}>
                  <Settings size={16} />
                  Settings
                </DropdownMenuItem>
                {accounts[0] && (
                  <DropdownMenuItem
                    disabled={accountBusy}
                    onSelect={() => void disconnectAccount(accounts[0].id)}
                  >
                    Disconnect this account
                  </DropdownMenuItem>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
          <div className="flex items-center gap-2">
            <Popover
              open={!!draft}
              onOpenChange={(open) => {
                if (!open && !calendar.save.isPending) setDraft(null);
              }}
            >
              <PopoverTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  aria-label="New event"
                  onClick={() => newEvent()}
                >
                  <SquarePen size={17} />
                </Button>
              </PopoverTrigger>
              <PopoverContent
                align="end"
                className="w-[350px] max-h-[85vh] overflow-auto overscroll-contain"
              >
                {draft && (
                  <EventEditor
                    draft={draft}
                    calendars={calendars.filter((item) => item.writable)}
                    saving={calendar.save.isPending}
                    onChange={setDraft}
                    onSave={() =>
                      void saveEvent(
                        draft,
                        calendar.data?.events.some(
                          (event) => event.eventId === draft.eventId,
                        )
                          ? draft.revision
                          : -1,
                        true,
                      )
                    }
                  />
                )}
              </PopoverContent>
            </Popover>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="secondary">
                  {view === "week" ? "Week" : "Month"}
                  <ChevronDown size={14} />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent>
                <DropdownMenuItem onSelect={() => setView("week")}>
                  Week <span className="ml-auto text-muted-foreground">W</span>
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => setView("month")}>
                  Month <span className="ml-auto text-muted-foreground">M</span>
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
            <Button variant="secondary" onClick={goToday}>
              Today
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Previous period"
              onClick={() => navigate(-1)}
            >
              <ChevronLeft size={19} />
            </Button>
            <Button
              variant="ghost"
              size="icon"
              aria-label="Next period"
              onClick={() => navigate(1)}
            >
              <ChevronRight size={19} />
            </Button>
          </div>
        </header>
        <h1 className="month-title">
          {monthLabel(view === "week" ? firstDay : selectedDate)}
        </h1>
        {calendar.isError && (
          <div role="alert" className="px-6 pb-3 text-amber-400 text-xs">
            {calendar.data ? "Offline snapshot · " : ""}
            {calendar.error.message}{" "}
            <button
              onClick={() => void calendar.refetch()}
              className="underline ml-2"
            >
              Retry
            </button>
          </div>
        )}
        {calendar.isPending && (
          <p className="px-6 text-xs text-muted-foreground">
            Loading calendars…
          </p>
        )}
        {view === "month" ? (
          <>
            <div className="grid grid-cols-7 text-center text-muted-foreground pb-3">
              {Array.from({ length: 7 }, (_, index) => (
                <span key={index}>{weekdayNames[(index + weekStart) % 7]}</span>
              ))}
            </div>
            <div
              className="month-grid"
              style={{
                gridTemplateRows: `repeat(${monthDates.length / 7}, minmax(110px,1fr))`,
              }}
            >
              {monthDates.map((date) => (
                <div
                  className="month-cell"
                  key={date}
                  onDoubleClick={() => newEvent(date)}
                >
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
          </>
        ) : (
          <div className="week-scroller" ref={scroller}>
            <div className="week-canvas">
              <div className="week-heading">
                <div className="text-[10px] text-muted-foreground pt-3 pl-3">
                  GMT+7
                </div>
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
                          (event.clientY -
                            event.currentTarget.getBoundingClientRect().top) /
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
                        (item) =>
                          item.eventId ===
                          event.dataTransfer.getData("text/plain"),
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
                        draggable={
                          !placement.event.readOnly && !calendar.save.isPending
                        }
                        onDragStart={(event) =>
                          event.dataTransfer.setData(
                            "text/plain",
                            placement.event.eventId,
                          )
                        }
                        onClick={() =>
                          openEvent(placement.event as EventRecord)
                        }
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
                          {clockTime(placement.event.start)}–
                          {clockTime(placement.event.end)}
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
                                document.removeEventListener(
                                  "pointerup",
                                  release,
                                );
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
                                        Date.parse(event.end) +
                                          difference * 60000,
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
        )}
      </main>
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="sm:max-w-[1000px] h-[min(720px,85vh)] p-0 gap-0 overflow-hidden">
          <div className="flex min-h-0">
            <nav className="w-56 bg-secondary p-5 space-y-2">
              <p className="text-xs text-muted-foreground mb-5">Account</p>
              <Button
                variant={settingsTab === "general" ? "secondary" : "ghost"}
                className="w-full justify-start"
                onClick={() => setSettingsTab("general")}
              >
                <Settings size={16} />
                General
              </Button>
              <Button
                variant={settingsTab === "accounts" ? "secondary" : "ghost"}
                className="w-full justify-start"
                onClick={() => setSettingsTab("accounts")}
              >
                <CalendarDays size={16} />
                Google accounts
              </Button>
            </nav>
            <section className="flex-1 p-9 overflow-auto">
              <DialogTitle>
                {settingsTab === "general"
                  ? "Calendar view"
                  : "Calendar accounts"}
              </DialogTitle>
              <DialogDescription className="mt-3">
                {settingsTab === "general"
                  ? "Choose how your calendar is displayed."
                  : "Manage your Google accounts and calendar permissions."}
              </DialogDescription>
              {settingsTab === "general" ? (
                <div className="space-y-8 mt-8">
                  <label className="max-w-60">
                    Appearance
                    <select
                      aria-label="Appearance"
                      value={theme}
                      onChange={(event) => {
                        selectTheme(event.target.value as CalendarTheme);
                        log("Appearance changed");
                      }}
                    >
                      <option value="system">System</option>
                      <option value="light">Light</option>
                      <option value="dark">Dark</option>
                    </select>
                  </label>
                  <label className="max-w-60">
                    Start week on
                    <select
                      value={weekStart}
                      onChange={(event) => {
                        const start = Number(event.target.value) as WeekStart;
                        setWeekStart(start);
                        localStorage.setItem(
                          "calendar:week-start",
                          String(start),
                        );
                        setFirstDay(startOfWeek(selectedDate, start));
                      }}
                    >
                      <option value="1">Monday</option>
                      <option value="0">Sunday</option>
                    </select>
                  </label>
                  <p className="text-muted-foreground">
                    Time zone: Asia/Jakarta (GMT+7)
                  </p>
                  <p className="text-muted-foreground">
                    Shortcuts: W — week · M — month · T — today
                  </p>
                  <Button
                    variant="secondary"
                    onClick={() => {
                      void storage.clear();
                      log("Offline snapshots cleared");
                    }}
                  >
                    Clear offline snapshots
                  </Button>
                </div>
              ) : (
                <div className="mt-8 space-y-5">
                  {accounts.map((account) => (
                    <div key={account.id} className="rounded-lg border p-4">
                      <strong>{account.name}</strong>
                      <p className="text-muted-foreground text-xs mt-1">
                        {account.email}
                      </p>
                      <p className="text-xs text-muted-foreground mt-3">
                        {account.message}
                      </p>
                      <div className="flex gap-2 mt-3">
                        {(!account.eventAccess || !account.writeAccess) && (
                          <Button
                            variant="secondary"
                            disabled={accountBusy}
                            onClick={() => void connectAccount(account.id)}
                          >
                            Approve calendar access
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          disabled={accountBusy}
                          onClick={() => void disconnectAccount(account.id)}
                        >
                          Disconnect
                        </Button>
                      </div>
                    </div>
                  ))}
                  <Button
                    variant="secondary"
                    disabled={accountBusy}
                    onClick={() => void connectAccount()}
                  >
                    <Plus size={16} />
                    Add Google account
                  </Button>
                  <p className="text-xs text-muted-foreground">
                    {calendar.data?.googleConnection.message}
                  </p>
                </div>
              )}
            </section>
          </div>
        </DialogContent>
      </Dialog>
      {message && (
        <div role="status" className="status-toast flex gap-4 items-center">
          {message}
          {!calendar.save.isPending && (
            <button aria-label="Dismiss message" onClick={() => setMessage("")}>
              <X size={14} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}
function EventEditor({
  draft,
  calendars,
  saving,
  onChange,
  onSave,
}: {
  draft: EventInput;
  calendars: { id: string; summary: string }[];
  saving: boolean;
  onChange: (event: EventInput) => void;
  onSave: () => void;
}) {
  const [error, setError] = useState("");
  const field = (patch: Partial<EventInput>) =>
    onChange({ ...draft, ...patch });
  const updateInstant = (key: "start" | "end", value: string) => {
    try {
      const [date, time] = value.split("T");
      field({ [key]: localInstant(date, time) });
      setError("");
    } catch {
      setError("Choose a valid date and time.");
    }
  };
  return (
    <form
      className="space-y-4"
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      <h2 className="font-semibold">
        {draft.readOnly ? "Event details" : "Event"}
      </h2>
      <fieldset disabled={saving || draft.readOnly} className="space-y-4">
        <input
          aria-label="Event title"
          placeholder="Add a title"
          value={draft.title}
          onChange={(event) => field({ title: event.target.value })}
          autoFocus
          required
        />
        <label>
          Calendar
          <select
            value={draft.calendarId}
            onChange={(event) => field({ calendarId: event.target.value })}
          >
            {calendars.map((calendar) => (
              <option key={calendar.id} value={calendar.id}>
                {calendar.summary}
              </option>
            ))}
          </select>
        </label>
        <label>
          Starts
          <input
            type="datetime-local"
            value={`${dateKey(new Date(draft.start))}T${clockTime(draft.start)}`}
            onChange={(event) => updateInstant("start", event.target.value)}
            required
          />
        </label>
        <label>
          Ends (exclusive for all-day)
          <input
            type="datetime-local"
            value={`${dateKey(new Date(draft.end))}T${clockTime(draft.end)}`}
            onChange={(event) => updateInstant("end", event.target.value)}
            required
          />
        </label>
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            className="w-auto"
            checked={draft.allDay}
            onChange={(event) => {
              const allDay = event.target.checked;
              field({
                allDay,
                ...(allDay
                  ? {
                      start: localInstant(
                        dateKey(new Date(draft.start)),
                        "00:00",
                      ),
                      end: localInstant(
                        addDays(dateKey(new Date(draft.start)), 1),
                        "00:00",
                      ),
                    }
                  : {}),
              });
            }}
          />
          All-day
        </label>
        <label>
          Location
          <input
            value={draft.location}
            onChange={(event) => field({ location: event.target.value })}
          />
        </label>
        <label>
          Notes
          <textarea
            rows={2}
            value={draft.description}
            onChange={(event) => field({ description: event.target.value })}
          />
        </label>
      </fieldset>
      {error && <p role="alert">{error}</p>}
      <p className="text-xs text-muted-foreground">
        Times in Asia/Jakarta. Recurring edits affect this occurrence.
      </p>
      {!draft.readOnly && (
        <Button type="submit" disabled={saving || !!error} className="w-full">
          {saving ? "Saving…" : "Save event"}
        </Button>
      )}
    </form>
  );
}
