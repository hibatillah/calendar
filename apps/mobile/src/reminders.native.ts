import type { CalendarEvent } from "@calendar/calendar";
import * as Notifications from "expo-notifications";
import {
  AndroidImportance,
  AndroidNotificationVisibility,
  IosAuthorizationStatus,
} from "expo-notifications";
import { Platform } from "react-native";
import { reminderPlan } from "./reminder-plan";

Notifications.setNotificationHandler({
  handleNotification: async () => ({
    shouldShowBanner: true,
    shouldShowList: true,
    shouldPlaySound: true,
    shouldSetBadge: false,
  }),
});

export async function enableReminders() {
  if (Platform.OS === "android")
    await Notifications.setNotificationChannelAsync("calendar-reminders", {
      name: "Event reminders",
      importance: AndroidImportance.DEFAULT,
      lockscreenVisibility: AndroidNotificationVisibility.PRIVATE,
    });

  const permission = await Notifications.requestPermissionsAsync();

  return (
    permission.granted || permission.ios?.status === IosAuthorizationStatus.PROVISIONAL
  );
}

let pending: Promise<void> = Promise.resolve();

export function syncReminders(events: CalendarEvent[], minutes: number | null) {
  pending = pending
    .catch(() => {})
    .then(async () => {
      const permission = await Notifications.getPermissionsAsync();
      const allowed =
        permission.granted ||
        permission.ios?.status === IosAuthorizationStatus.PROVISIONAL;
      const plan = reminderPlan(events, allowed ? minutes : null);
      const existing = await Notifications.getAllScheduledNotificationsAsync();
      const wanted = new Map(plan.map((reminder) => [reminder.identifier, reminder]));

      for (const notification of existing) {
        if (!notification.identifier.startsWith("calendar:")) continue;

        const target = wanted.get(notification.identifier);

        if (target && notification.content.data?.fireAt === target.fireAt)
          wanted.delete(notification.identifier);
        else
          await Notifications.cancelScheduledNotificationAsync(notification.identifier);
      }

      for (const reminder of wanted.values())
        await Notifications.scheduleNotificationAsync({
          identifier: reminder.identifier,
          content: {
            title: "Upcoming calendar event",
            body:
              minutes === 0
                ? "Your event is starting."
                : `Your event starts in ${minutes} minutes.`,
            sound: "default",
            data: {
              date: reminder.date,
              eventId: reminder.eventId,
              fireAt: reminder.fireAt,
            },
          },
          trigger: {
            type: Notifications.SchedulableTriggerInputTypes.DATE,
            date: new Date(reminder.fireAt),
            channelId: "calendar-reminders",
          },
        });
    });

  return pending;
}

export function watchReminderOpen(onOpen: (date: string) => void) {
  const open = (response: Notifications.NotificationResponse | null) => {
    const date = response?.notification.request.content.data?.date;

    if (typeof date === "string" && /^\d{4}-\d{2}-\d{2}$/.test(date)) onOpen(date);
  };

  void Notifications.getLastNotificationResponseAsync().then(open);

  const subscription = Notifications.addNotificationResponseReceivedListener(open);

  return () => subscription.remove();
}
