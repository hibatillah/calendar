import { useEffect, useMemo, useRef, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  AppState,
  Platform,
  Linking,
  FlatList,
  Pressable,
  RefreshControl,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import { StatusBar } from "expo-status-bar";
import { SafeAreaView } from "react-native-safe-area-context";
import * as Crypto from "expo-crypto";
import * as Haptics from "expo-haptics";
import { readPreferences, writePreferences } from "./preferences";
import { enableReminders, syncReminders, watchReminderOpen } from "./reminders";
import { useQueryClient } from "@tanstack/react-query";
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Plus,
  Settings,
} from "lucide-react-native";
import { colors } from "@calendar/design-tokens";
import {
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
  type WeekStart,
} from "@calendar/calendar";
import { createTransport, type EventInput } from "@calendar/api";
import { useCalendar } from "@calendar/api/react";
import { snapshotStorage } from "./storage";
import {
  clearConnection,
  loadConnection,
  saveConnection,
  type ConnectionSettings,
} from "./connection";
import {
  Action,
  Label,
  textStyle,
  inputStyle,
  Sheet,
  MobileEditor,
} from "./components";

export function CalendarScreen() {
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
  const [hiddenCalendars, setHiddenCalendars] = useState<Set<string>>(
    new Set(),
  );
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
    if (!preferencesLoaded || !connectionLoaded || calendar.save.isPending)
      return;
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
  }, [
    connection,
    reminderMinutes,
    refreshCalendar,
    savingEvent,
    refreshUpcoming,
  ]);
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
  const agenda = eventsOnDay(events, selectedDate);
  const touchStart = useRef(0);
  const log = (action: string) =>
    setActivity((previous) =>
      [`${new Date().toLocaleTimeString()} · ${action}`, ...previous].slice(
        0,
        60,
      ),
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
      .catch(() =>
        setMessage("Could not restore the connection. Open Settings."),
      )
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
      color:
        parseInt(selectedCalendar.backgroundColor.replace("#", ""), 16) ||
        5614047,
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
    if (
      !event.title.trim() ||
      Date.parse(event.end) <= Date.parse(event.start)
    ) {
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
        void Haptics.notificationAsync(
          Haptics.NotificationFeedbackType.Success,
        ).catch(() => {});
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
      Alert.alert(
        "Discard changes?",
        "Your unsaved event changes will be lost.",
        [
          { text: "Keep editing", style: "cancel" },
          {
            text: "Discard",
            style: "destructive",
            onPress: () => setDraft(null),
          },
        ],
      );
  };
  return (
    <SafeAreaView style={{ flex: 1, backgroundColor: colors.background }}>
      <StatusBar style="light" />
      <View className="flex-row items-center justify-between px-5 py-2">
        <Action label="Settings" onPress={() => setSettingsOpen(true)}>
          <Settings color={colors.text} size={23} />
        </Action>
        <Text style={[textStyle, { fontSize: 23, fontWeight: "600" }]}>
          {monthLabel(selectedDate)}
        </Text>
        <Action label="Today" onPress={() => setSelectedDate(dateKey())}>
          <CalendarDays color={colors.text} size={23} />
        </Action>
      </View>
      <View className="flex-row items-center justify-between px-6">
        <Action
          label="Previous month"
          onPress={() => setSelectedDate(addMonths(selectedDate, -1))}
        >
          <ChevronLeft color={colors.muted} />
        </Action>
        <Label muted>Asia/Jakarta</Label>
        <Action
          label="Next month"
          onPress={() => setSelectedDate(addMonths(selectedDate, 1))}
        >
          <ChevronRight color={colors.muted} />
        </Action>
      </View>
      <View
        className="px-4 pb-4"
        onTouchStart={(event) => {
          touchStart.current = event.nativeEvent.pageX;
        }}
        onTouchEnd={(event) => {
          const distance = event.nativeEvent.pageX - touchStart.current;
          if (Math.abs(distance) > 70)
            setSelectedDate((previous) =>
              addMonths(previous, distance < 0 ? 1 : -1),
            );
        }}
      >
        <View className="flex-row mb-2">
          {Array.from({ length: 7 }, (_, index) => (
            <Text
              key={index}
              style={[
                textStyle,
                {
                  width: "14.285714%",
                  textAlign: "center",
                  color: colors.muted,
                  fontSize: 11,
                },
              ]}
            >
              {
                ["SUN", "MON", "TUE", "WED", "THU", "FRI", "SAT"][
                  (weekStart + index) % 7
                ]
              }
            </Text>
          ))}
        </View>
        <View className="flex-row flex-wrap">
          {dates.map((date) => {
            const dailyEvents = eventsOnDay(events, date);
            return (
              <Pressable
                accessibilityRole="button"
                accessibilityLabel={`${date}, ${dailyEvents.length} events`}
                accessibilityState={{ selected: date === selectedDate }}
                key={date}
                onPress={() => {
                  setSelectedDate(date);
                  if (Platform.OS !== "web")
                    void Haptics.selectionAsync().catch(() => {});
                }}
                style={{
                  width: "14.285714%",
                  height: 61,
                  padding: 4,
                  borderWidth: 1,
                  borderRadius: 8,
                  borderColor: date === selectedDate ? "#777" : "transparent",
                  opacity:
                    date.slice(0, 7) === selectedDate.slice(0, 7) ? 1 : 0.35,
                }}
              >
                <Text
                  style={[
                    textStyle,
                    {
                      alignSelf: "center",
                      textAlign: "center",
                      fontSize: 17,
                      paddingHorizontal: 5,
                      borderRadius: 5,
                      backgroundColor:
                        date === dateKey() ? colors.today : "transparent",
                    },
                  ]}
                >
                  {Number(date.slice(-2))}
                </Text>
                <View className="gap-0.5 mt-1">
                  {dailyEvents.slice(0, 3).map((event) => (
                    <View
                      key={event.eventId}
                      style={{
                        height: 3,
                        borderRadius: 3,
                        backgroundColor: eventColor(event.color),
                      }}
                    />
                  ))}
                  {dailyEvents.length > 3 && (
                    <Text
                      style={[
                        textStyle,
                        {
                          fontSize: 8,
                          textAlign: "right",
                          color: colors.muted,
                        },
                      ]}
                    >
                      +{dailyEvents.length - 3}
                    </Text>
                  )}
                </View>
              </Pressable>
            );
          })}
        </View>
      </View>
      <View className="border-t border-neutral-800 px-6 py-4 flex-row justify-between">
        <Label>
          {dayLabel(selectedDate)} · {agenda.length}{" "}
          {agenda.length === 1 ? "event" : "events"}
        </Label>
        {calendar.isFetching && (
          <ActivityIndicator size="small" color={colors.muted} />
        )}
      </View>
      {calendar.isError && (
        <Pressable
          onPress={() => void calendar.refetch()}
          className="px-6 pb-3"
        >
          <Text style={[textStyle, { fontSize: 12, color: "#d6b079" }]}>
            {calendar.data ? "Offline snapshot. " : ""}
            {calendar.error.message} Tap to retry.
          </Text>
        </Pressable>
      )}
      <FlatList
        data={agenda}
        keyExtractor={(event) => event.eventId}
        style={{ flex: 1 }}
        contentContainerStyle={{ paddingBottom: 95 }}
        refreshControl={
          <RefreshControl
            refreshing={calendar.isFetching}
            onRefresh={() => void calendar.refetch()}
            tintColor={colors.text}
          />
        }
        ListEmptyComponent={
          <View className="px-6 py-8">
            <Label muted>
              {connection
                ? "No events for this day."
                : "Connect your calendar in Settings to see your events."}
            </Label>
          </View>
        }
        renderItem={({ item }) => (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel={`Open ${item.title}`}
            onPress={() => {
              const event = item;
              setDraft(event);
              setDraftDirty(false);
              setCommand(null);
            }}
            className="flex-row gap-4 px-6 py-5 border-b border-neutral-800"
          >
            <View style={{ width: 48 }}>
              <Text style={[textStyle, { fontSize: 14 }]}>
                {item.allDay ? "All-day" : clockTime(item.start)}
              </Text>
            </View>
            <View
              style={{
                backgroundColor: eventColor(item.color),
                width: 4,
                borderRadius: 3,
              }}
            />
            <View className="flex-1 gap-1">
              <Text style={[textStyle, { fontSize: 17 }]}>{item.title}</Text>
              <Text style={[textStyle, { color: colors.muted, fontSize: 12 }]}>
                {item.allDay
                  ? "All-day"
                  : `${clockTime(item.start)} – ${clockTime(item.end)}`}
                {item.location ? ` · ${item.location}` : ""}
              </Text>
            </View>
          </Pressable>
        )}
      />
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Create event"
        onPress={newEvent}
        className="absolute right-6 bottom-8 w-16 h-16 rounded-full items-center justify-center bg-neutral-800"
      >
        <Plus color={colors.accent} size={32} />
      </Pressable>
      <Sheet
        title="Settings"
        visible={settingsOpen}
        close={() => setSettingsOpen(false)}
      >
        <ScrollView contentContainerStyle={{ padding: 24, gap: 20 }}>
          <Label>Start week on</Label>
          <View className="flex-row gap-4">
            {([1, 0] as WeekStart[]).map((start) => (
              <Pressable
                key={start}
                onPress={() => setWeekStart(start)}
                className="px-4 py-3 rounded-lg"
                style={{
                  backgroundColor: weekStart === start ? "#444" : "#292929",
                }}
              >
                <Label>{start ? "Monday" : "Sunday"}</Label>
              </Pressable>
            ))}
          </View>
          <Label>Reminders</Label>
          <Label muted>
            Private local alerts for the next 50 timed events. Open Calendar
            regularly to sync changes; delivery follows your device notification
            settings.
          </Label>
          <View className="flex-row flex-wrap gap-2">
            {[null, 0, 5, 10, 15, 30].map((minutes) => (
              <Pressable
                key={String(minutes)}
                accessibilityRole="button"
                onPress={async () => {
                  try {
                    if (minutes !== null && !(await enableReminders())) {
                      setMessage(
                        "Allow notifications in system Settings. Reminders require the native app.",
                      );
                      return;
                    }
                    setReminderMinutes(minutes);
                  } catch {
                    setMessage("Could not enable reminders.");
                  }
                }}
                style={{
                  padding: 12,
                  borderRadius: 8,
                  backgroundColor:
                    reminderMinutes === minutes ? "#444" : "#292929",
                }}
              >
                <Label>
                  {minutes === null
                    ? "Off"
                    : minutes === 0
                      ? "At start"
                      : `${minutes} min`}
                </Label>
              </Pressable>
            ))}
          </View>
          <Pressable
            accessibilityRole="button"
            onPress={() => {
              if (Platform.OS !== "web") void Linking.openSettings();
            }}
          >
            <Label muted>Open system notification settings</Label>
          </Pressable>
          <Label>Local backend connection</Label>
          <Label muted>
            For Android USB/emulator, run adb reverse tcp:47821 tcp:47821 on
            your computer. Google sign-in remains in the desktop app for this
            local preview.
          </Label>
          <TextInput
            accessibilityLabel="Backend URL"
            style={inputStyle}
            autoCapitalize="none"
            autoCorrect={false}
            value={endpoint}
            onChangeText={setEndpoint}
          />
          <TextInput
            accessibilityLabel="Local session token"
            placeholder="Local session token"
            placeholderTextColor={colors.muted}
            style={inputStyle}
            secureTextEntry
            autoCapitalize="none"
            autoCorrect={false}
            value={token}
            onChangeText={setToken}
          />
          <Pressable
            className="bg-neutral-700 rounded-lg p-4"
            onPress={() => void updateConnection()}
          >
            <Label>Save connection</Label>
          </Pressable>
          {accounts.map((account) => (
            <View key={account.id} className="gap-3">
              <Text style={[textStyle, { color: colors.muted, fontSize: 12 }]}>
                {account.email}
              </Text>
              {account.calendars.map((item) => (
                <Pressable
                  key={item.id}
                  onPress={() =>
                    setHiddenCalendars((previous) => {
                      const next = new Set(previous);
                      if (next.has(item.id)) next.delete(item.id);
                      else next.add(item.id);
                      return next;
                    })
                  }
                  className="flex-row items-center gap-3 py-1"
                >
                  <View
                    style={{
                      width: 13,
                      height: 13,
                      borderRadius: 4,
                      backgroundColor: hiddenCalendars.has(item.id)
                        ? "#555"
                        : item.backgroundColor,
                    }}
                  />
                  <Label>{item.summary}</Label>
                </Pressable>
              ))}
            </View>
          ))}
          <Pressable
            className="py-3"
            onPress={async () => {
              await clearConnection();
              await queryClient.cancelQueries();
              queryClient.clear();
              await snapshotStorage.clear();
              await syncReminders([], null);
              setHiddenCalendars(new Set());
              setConnection(null);
              setToken("");
              log("Local connection removed");
            }}
          >
            <Label>Remove local connection and offline data</Label>
          </Pressable>
          <Pressable
            className="py-3"
            onPress={() => {
              setSettingsOpen(false);
              setActivityOpen(true);
            }}
          >
            <Label>Activity</Label>
          </Pressable>
          <Label muted>
            Credentials use SecureStore on Android/iOS. Calendar snapshots stay
            on this device. Public hosted sign-in is not enabled yet.
          </Label>
          {!!message && <Label>{message}</Label>}
        </ScrollView>
      </Sheet>
      <Sheet title="Event" visible={!!draft} close={closeEditor}>
        {draft && (
          <MobileEditor
            key={draft.eventId}
            draft={draft}
            onChange={(event) => {
              setDraftDirty(true);
              setDraft(event);
            }}
            onDirty={() => setDraftDirty(true)}
            editable={!calendar.isError && !!connection}
            onSave={(event) => void save(event)}
            saving={calendar.save.isPending}
            calendars={calendars.filter((item) => item.writable)}
          />
        )}
        <Text style={[textStyle, { padding: 20, color: "#d6b079" }]}>
          {message}
        </Text>
      </Sheet>
      <Sheet
        title="Activity"
        visible={activityOpen}
        close={() => setActivityOpen(false)}
      >
        <ScrollView contentContainerStyle={{ padding: 24, gap: 16 }}>
          {activity.length ? (
            activity.map((entry, index) => (
              <Label key={index} muted>
                {entry}
              </Label>
            ))
          ) : (
            <Label muted>No activity this session.</Label>
          )}
        </ScrollView>
      </Sheet>
      {!!message && !draft && !settingsOpen && (
        <Pressable
          onPress={() => setMessage("")}
          className="absolute bottom-28 left-5 right-5 bg-neutral-700 p-4 rounded-xl"
        >
          <Text accessibilityRole="alert" style={textStyle}>
            {message}
          </Text>
        </Pressable>
      )}
    </SafeAreaView>
  );
}
