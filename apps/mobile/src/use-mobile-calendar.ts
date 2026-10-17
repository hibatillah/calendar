import { createTransport, type EventInput } from "@calendar/api";
import { useCalendar } from "@calendar/api/react";
import {
  dateKey,
  DISPLAY_ZONE,
  eventsOnDay,
  localInstant,
  monthDays,
  type WeekStart,
} from "@calendar/calendar";
import { useQueryClient } from "@tanstack/react-query";
import * as Crypto from "expo-crypto";
import * as Haptics from "expo-haptics";
import { useEffect, useMemo, useRef, useState } from "react";
import { Alert, AppState, Platform } from "react-native";
import { loadConnection, saveConnection, type ConnectionSettings } from "./connection";
import { readPreferences, writePreferences } from "./preferences";
import { syncReminders, watchReminderOpen } from "./reminders";
import { snapshotStorage } from "./storage";

export function useMobileCalendar() {
  const [selectedDate, setSelectedDate] = useState(dateKey);
  const [weekStart, setWeekStart] = useState<WeekStart>(1);
  const [preferencesLoaded, setPreferencesLoaded] = useState(false);
  const [reminderMinutes, setReminderMinutes] = useState<number | null>(null);
  const [draftDirty, setDraftDirty] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [connection, setConnection] = useState<ConnectionSettings | null>(null);
  const [endpoint, setEndpoint] = useState("http://127.0.0.1:47821/graphql");
  const [token, setToken] = useState("");
  const [connectionLoaded, setConnectionLoaded] = useState(false);
  const [draft, setDraft] = useState<EventInput | null>(null);
  const [command, setCommand] = useState<{
    event: EventInput;
    expectedRevision: number;
    operationId: string;
  } | null>(null);
  const [message, setMessage] = useState("");
  const [hiddenCalendars, setHiddenCalendars] = useState<Set<string>>(new Set());
  const [activity, setActivity] = useState<string[]>([]);
  const [activityOpen, setActivityOpen] = useState(false);
  const queryClient = useQueryClient();
  const dates = useMemo(
    () => monthDays(selectedDate, weekStart),
    [selectedDate, weekStart],
  );
  const transport = useMemo(
    () =>
      createTransport(
        connection?.endpoint ?? "http://127.0.0.1:1/unconfigured",
        connection?.token,
      ),
    [connection],
  );
  const calendar = useCalendar(
    transport,
    dates[0],
    snapshotStorage,
    connectionLoaded && !!connection,
  );
  const upcoming = useCalendar(
    transport,
    dateKey(),
    snapshotStorage,
    connectionLoaded && !!connection && reminderMinutes !== null,
  );

  useEffect(() => {
    readPreferences()
      .then((preferences) => {
        setWeekStart(preferences.weekStart);
        setHiddenCalendars(new Set(preferences.hiddenCalendars));
        setReminderMinutes(preferences.reminderMinutes);
      })
      .catch(() => setMessage("Could not restore preferences."))
      .finally(() => setPreferencesLoaded(true));

    return watchReminderOpen(setSelectedDate);
  }, []);
  useEffect(() => {
    if (preferencesLoaded)
      void writePreferences({
        weekStart,
        hiddenCalendars: [...hiddenCalendars],
        reminderMinutes,
      }).catch(() => setMessage("Could not save preferences."));
  }, [preferencesLoaded, weekStart, hiddenCalendars, reminderMinutes]);
  useEffect(() => {
    if (!preferencesLoaded || !connectionLoaded || calendar.save.isPending) return;

    if (connection && reminderMinutes !== null && !upcoming.data) return;

    const visible = (upcoming.data?.events ?? []).filter(
      (event) => !hiddenCalendars.has(event.calendarId),
    );

    void syncReminders(visible, connection ? reminderMinutes : null).catch(() =>
      setMessage("Could not update reminders. Check notification settings."),
    );
  }, [
    preferencesLoaded,
    connectionLoaded,
    connection,
    reminderMinutes,
    hiddenCalendars,
    upcoming.data,
    calendar.save.isPending,
  ]);

  const refreshCalendar = calendar.refetch;
  const refreshUpcoming = upcoming.refetch;
  const savingEvent = calendar.save.isPending;

  useEffect(() => {
    const subscription = AppState.addEventListener("change", (state) => {
      if (state === "active" && connection && !savingEvent) {
        void refreshCalendar();

        if (reminderMinutes !== null) void refreshUpcoming();
      }
    });

    return () => subscription.remove();
  }, [connection, reminderMinutes, refreshCalendar, savingEvent, refreshUpcoming]);

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
  const agenda = eventsOnDay(events, selectedDate);
  const touchStartRef = useRef(0);
  const log = (action: string) =>
    setActivity((previous) =>
      [`${new Date().toLocaleTimeString()} · ${action}`, ...previous].slice(0, 60),
    );

  useEffect(() => {
    loadConnection()
      .then((value) => {
        if (value) {
          setConnection(value);
          setEndpoint(value.endpoint);
          setToken(value.token);
        }
      })
      .catch(() => setMessage("Could not restore the connection. Open Settings."))
      .finally(() => setConnectionLoaded(true));
  }, []);

  const newEvent = () => {
    if (calendar.isError) {
      setMessage("Offline · viewing only. Reconnect before editing.");

      return;
    }

    const selectedCalendar =
      calendars.find((item) => item.writable && item.primary) ??
      calendars.find((item) => item.writable);

    if (!selectedCalendar) {
      setMessage(
        "Connect the backend and approve Google calendar editing in the desktop app first.",
      );
      setSettingsOpen(true);

      return;
    }

    const start = localInstant(selectedDate, "09:00");

    setCommand(null);
    setDraftDirty(false);
    setDraft({
      eventId: Crypto.randomUUID(),
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
  const save = async (event: EventInput) => {
    if (calendar.save.isPending) return;

    setDraft(event);

    if (calendar.isError || !connection) {
      setMessage("Reconnect before saving. Your draft is retained.");

      return;
    }

    if (!event.title.trim() || Date.parse(event.end) <= Date.parse(event.start)) {
      setMessage("Add a title and choose an end after the start.");

      return;
    }

    const nextCommand =
      command && JSON.stringify(command.event) === JSON.stringify(event)
        ? command
        : {
            event,
            operationId: Crypto.randomUUID(),
            expectedRevision: calendar.data?.events.some(
              (previous) => previous.eventId === event.eventId,
            )
              ? event.revision
              : -1,
          };

    setCommand(nextCommand);
    log("Saving event");
    setMessage("Saving…");

    try {
      await calendar.save.mutateAsync(nextCommand);

      if (Platform.OS !== "web")
        void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success).catch(
          () => {},
        );

      setDraft(null);
      setCommand(null);
      setMessage("Saved");
      log("Event saved");
    } catch (error) {
      setMessage((error as Error).message);
      log("Event save failed");
    }
  };
  const updateConnection = async () => {
    try {
      const settings = { endpoint: endpoint.trim(), token: token.trim() };

      if (!settings.token) throw new Error("Enter the local session token.");

      await saveConnection(settings);
      await queryClient.cancelQueries();
      queryClient.clear();
      await snapshotStorage.clear();
      await syncReminders([], null);
      setConnection(settings);
      setSettingsOpen(false);
      log("Connection updated");
    } catch (error) {
      setMessage((error as Error).message);
    }
  };
  const closeEditor = () => {
    if (calendar.save.isPending) return;

    if (!draftDirty) {
      setDraft(null);

      return;
    }

    if (Platform.OS === "web") {
      if (window.confirm("Discard your unsaved changes?")) setDraft(null);
    } else
      Alert.alert("Discard changes?", "Your unsaved event changes will be lost.", [
        { text: "Keep editing", style: "cancel" },
        {
          text: "Discard",
          style: "destructive",
          onPress: () => setDraft(null),
        },
      ]);
  };

  return {
    selectedDate,
    setSelectedDate,
    weekStart,
    setWeekStart,
    reminderMinutes,
    setReminderMinutes,
    setDraftDirty,
    settingsOpen,
    setSettingsOpen,
    connection,
    setConnection,
    endpoint,
    setEndpoint,
    token,
    setToken,
    draft,
    setDraft,
    setCommand,
    message,
    setMessage,
    hiddenCalendars,
    setHiddenCalendars,
    activity,
    activityOpen,
    setActivityOpen,
    queryClient,
    dates,
    calendar,
    accounts,
    calendars,
    events,
    agenda,
    touchStartRef,
    log,
    newEvent,
    save,
    updateConnection,
    closeEditor,
  };
}

export type MobileCalendarController = ReturnType<typeof useMobileCalendar>;
