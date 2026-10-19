import { type WeekStart } from "@calendar/calendar";
import { colors } from "@calendar/design-tokens";
import {
  Linking,
  Platform,
  Pressable,
  ScrollView,
  Text,
  TextInput,
  View,
} from "react-native";
import { inputStyle, Label, Sheet, textStyle } from "./components";
import { clearConnection } from "./connection";
import { enableReminders, syncReminders } from "./reminders";
import { snapshotStorage } from "./storage";
import type { MobileCalendarController } from "./use-mobile-calendar";

export function MobileSettings({ state }: { state: MobileCalendarController }) {
  const {
    weekStart,
    setWeekStart,
    reminderMinutes,
    setReminderMinutes,
    settingsOpen,
    setSettingsOpen,
    setConnection,
    endpoint,
    setEndpoint,
    token,
    setToken,
    message,
    setMessage,
    hiddenCalendars,
    setHiddenCalendars,
    setActivityOpen,
    queryClient,
    accounts,
    log,
    updateConnection,
  } = state;

  return (
    <Sheet title="Settings" visible={settingsOpen} close={() => setSettingsOpen(false)}>
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
          Private local alerts for the next 50 timed events. Open Calendar regularly to
          sync changes; delivery follows your device notification settings.
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
                backgroundColor: reminderMinutes === minutes ? "#444" : "#292929",
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
          For Android USB/emulator, run adb reverse tcp:47821 tcp:47821 on your
          computer. Google sign-in remains in the desktop app for this local preview.
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
          Credentials use SecureStore on Android/iOS. Calendar snapshots stay on this
          device. Public hosted sign-in is not enabled yet.
        </Label>
        {!!message && <Label>{message}</Label>}
      </ScrollView>
    </Sheet>
  );
}
