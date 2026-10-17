import {
  addMonths,
  clockTime,
  dateKey,
  dayLabel,
  eventColor,
  monthLabel,
} from "@calendar/calendar";
import { colors } from "@calendar/design-tokens";
import { StatusBar } from "expo-status-bar";
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Plus,
  Settings,
} from "lucide-react-native";
import {
  ActivityIndicator,
  FlatList,
  Pressable,
  RefreshControl,
  ScrollView,
  Text,
  View,
} from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { Action, Label, Sheet, textStyle } from "./components";
import { MobileEditor } from "./mobile-editor";
import { MobileMonth } from "./mobile-month";
import { MobileSettings } from "./mobile-settings";
import { useMobileCalendar } from "./use-mobile-calendar";

export function CalendarScreen() {
  const calendarState = useMobileCalendar();
  const {
    selectedDate,
    setSelectedDate,
    setDraftDirty,
    settingsOpen,
    setSettingsOpen,
    connection,
    draft,
    setDraft,
    setCommand,
    message,
    setMessage,
    activity,
    activityOpen,
    setActivityOpen,
    calendar,
    calendars,
    agenda,
    newEvent,
    save,
    closeEditor,
  } = calendarState;

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
      <MobileMonth state={calendarState} />
      <View className="border-t border-neutral-800 px-6 py-4 flex-row justify-between">
        <Label>
          {dayLabel(selectedDate)} · {agenda.length}{" "}
          {agenda.length === 1 ? "event" : "events"}
        </Label>
        {calendar.isFetching && <ActivityIndicator size="small" color={colors.muted} />}
      </View>
      {calendar.isError && (
        <Pressable onPress={() => void calendar.refetch()} className="px-6 pb-3">
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
      <MobileSettings state={calendarState} />
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
        <Text style={[textStyle, { padding: 20, color: "#d6b079" }]}>{message}</Text>
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
