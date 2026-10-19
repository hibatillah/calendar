import {
  ConnectCalendarAccountDocument,
  createTransport,
  DisconnectCalendarAccountDocument,
  type EventInput,
  type EventRecord,
} from "@calendar/api";
import { useCalendar } from "@calendar/api/react";
import {
  addDays,
  addMonths,
  dateKey,
  DISPLAY_ZONE,
  localInstant,
  monthDays,
  startOfWeek,
  type WeekStart,
} from "@calendar/calendar";
import { useQueryClient } from "@tanstack/react-query";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { storage } from "./snapshot-storage";
import { useCalendarTheme } from "./use-calendar-theme";

const transport = createTransport("/api/graphql");

export function useCalendarController() {
  const { theme, selectTheme } = useCalendarTheme();
  const [today, setToday] = useState(dateKey);
  const [selectedDate, setSelectedDate] = useState(today);
  const [firstDay, setFirstDay] = useState(() => startOfWeek(today));
  const [weekStart, setWeekStart] = useState<WeekStart>(1);
  const [view, setView] = useState<"week" | "month">("week");
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsTab, setSettingsTab] = useState<"general" | "accounts">("general");
  const [activityOpen, setActivityOpen] = useState(false);
  const [activity, setActivity] = useState<string[]>([]);
  const [hiddenCalendars, setHiddenCalendars] = useState<Set<string>>(new Set());
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
      writable: account.writeAccess && ["owner", "writer"].includes(item.accessRole),
    })),
  );
  const events = (calendar.data?.events ?? []).filter(
    (event) => !hiddenCalendars.has(event.calendarId),
  );
  const visibleDates =
    view === "month"
      ? monthDates
      : Array.from({ length: 7 }, (_, index) => addDays(firstDay, index));
  const timelineScrollRef = useRef<HTMLDivElement>(null);
  const scrollAccumulator = useRef(0);
  const log = (action: string) =>
    setActivity((previous) =>
      [`${new Date().toLocaleTimeString()} · ${action}`, ...previous].slice(0, 80),
    );

  useEffect(() => {
    const saved = localStorage.getItem("calendar:week-start");

    if (saved === "0" || saved === "1") {
      const start = Number(saved) as WeekStart;

      // oxlint-disable-next-line react/set-state-in-effect -- Restore browser preferences after SSR hydration.
      setWeekStart(start);
      setFirstDay(startOfWeek(dateKey(), start));
    }

    const interval = setInterval(() => setToday(dateKey()), 60_000);

    return () => clearInterval(interval);
  }, []);

  const goToday = useCallback(() => {
    const current = dateKey();

    setSelectedDate(current);
    setFirstDay(startOfWeek(current, weekStart));
  }, [weekStart]);

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
  }, [goToday, draft, settingsOpen]);
  useEffect(() => {
    const element = timelineScrollRef.current;

    if (!element) return;

    element.scrollTop = 7 * 60;

    const scroll = (event: WheelEvent) => {
      if (Math.abs(event.deltaX) <= Math.abs(event.deltaY) && !event.shiftKey) return;

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
    if (view === "month") setSelectedDate((previous) => addMonths(previous, direction));
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
      setMessage("Connect a Google account and approve calendar editing first.");

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
      color: parseInt(selectedCalendar.backgroundColor.replace("#", ""), 16) || 5614047,
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

    if (!event.title.trim() || Date.parse(event.end) <= Date.parse(event.start)) {
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

  return {
    theme,
    selectTheme,
    today,
    selectedDate,
    setSelectedDate,
    firstDay,
    setFirstDay,
    weekStart,
    setWeekStart,
    view,
    setView,
    sidebarOpen,
    setSidebarOpen,
    settingsOpen,
    setSettingsOpen,
    settingsTab,
    setSettingsTab,
    activityOpen,
    setActivityOpen,
    activity,
    hiddenCalendars,
    setHiddenCalendars,
    draft,
    setDraft,
    message,
    setMessage,
    accountBusy,
    monthDates,
    calendar,
    accounts,
    calendars,
    events,
    visibleDates,
    timelineScrollRef,
    log,
    goToday,
    navigate,
    newEvent,
    saveEvent,
    connectAccount,
    disconnectAccount,
    openEvent,
  };
}

export type CalendarController = ReturnType<typeof useCalendarController>;
