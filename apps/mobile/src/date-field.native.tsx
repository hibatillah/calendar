import { useState } from "react";
import { Platform, Pressable, Text, View } from "react-native";
import DateTimePicker from "@react-native-community/datetimepicker";
import { clockTime, dateKey, DISPLAY_ZONE, localInstant } from "@calendar/calendar";
import type { DateFieldProps } from "./date-field";

export function DateField({
  label,
  value,
  onChange,
  disabled,
  allDay,
}: DateFieldProps) {
  const [mode, setMode] = useState<"date" | "time" | null>(null);
  const [date, time] = value.trim().split(/\s+/);
  const selected = new Date(localInstant(date, time || "00:00"));

  return (
    <View style={{ gap: 8 }}>
      <View style={{ flexDirection: "row", gap: 12 }}>
        {(["date", ...(allDay ? [] : ["time"])] as ("date" | "time")[]).map((part) => (
          <Pressable
            key={part}
            accessibilityRole="button"
            accessibilityLabel={`${label}: ${part}`}
            disabled={disabled}
            onPress={() => setMode(part)}
            style={{
              padding: 12,
              backgroundColor: "#303030",
              borderRadius: 8,
            }}
          >
            <Text style={{ fontFamily: "Inter", color: "#e9e9e9" }}>
              {part === "date" ? date : time}
            </Text>
          </Pressable>
        ))}
      </View>
      {mode && !disabled && (
        <DateTimePicker
          value={selected}
          mode={mode}
          timeZoneName={DISPLAY_ZONE}
          themeVariant="dark"
          display={Platform.OS === "ios" ? "spinner" : "default"}
          is24Hour
          onChange={(event, nextDate) => {
            if (Platform.OS === "android") setMode(null);

            if (event.type === "set" && nextDate)
              onChange(
                `${dateKey(nextDate)} ${allDay ? "00:00" : clockTime(nextDate.toISOString())}`,
              );
          }}
        />
      )}
      {mode && Platform.OS === "ios" && (
        <Pressable
          accessibilityRole="button"
          onPress={() => setMode(null)}
          style={{ padding: 12 }}
        >
          <Text style={{ color: "#9ac4f8", textAlign: "right" }}>Done</Text>
        </Pressable>
      )}
    </View>
  );
}
