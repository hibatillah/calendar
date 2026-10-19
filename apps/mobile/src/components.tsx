import { useState } from "react";
import {
  Modal,
  KeyboardAvoidingView,
  Platform,
  Pressable,
  ScrollView,
  Switch,
  Text,
  TextInput,
  View,
} from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";
import { X } from "lucide-react-native";
import { colors } from "@calendar/design-tokens";
import { addDays, clockTime, dateKey, localInstant } from "@calendar/calendar";
import type { EventInput } from "@calendar/api";
import { DateField } from "./date-field";

export const textStyle = { fontFamily: "Inter", color: colors.text };
export const inputStyle = {
  ...textStyle,
  backgroundColor: "#303030",
  borderRadius: 8,
  padding: 12,
};

export function Label({
  children,
  muted = false,
}: {
  children: React.ReactNode;
  muted?: boolean;
}) {
  return <Text style={[textStyle, muted && { color: colors.muted }]}>{children}</Text>;
}

export function Action({
  label,
  onPress,
  children,
  disabled,
}: {
  label: string;
  onPress: () => void;
  children: React.ReactNode;
  disabled?: boolean;
}) {
  return (
    <Pressable
      accessibilityRole="button"
      accessibilityLabel={label}
      onPress={onPress}
      disabled={disabled}
      className="min-h-11 min-w-11 items-center justify-center rounded-xl active:bg-neutral-800"
      style={{ opacity: disabled ? 0.4 : 1 }}
    >
      {children}
    </Pressable>
  );
}

export function Sheet({
  title,
  visible,
  close,
  children,
}: {
  title: string;
  visible: boolean;
  close: () => void;
  children: React.ReactNode;
}) {
  return (
    <Modal
      visible={visible}
      animationType="slide"
      presentationStyle="pageSheet"
      onRequestClose={close}
    >
      <SafeAreaView style={{ flex: 1, backgroundColor: colors.sidebar }}>
        <View className="flex-row justify-between items-center px-5 py-3">
          <Text style={[textStyle, { fontSize: 22 }]}>{title}</Text>
          <Action label={`Close ${title.toLowerCase()}`} onPress={close}>
            <X color={colors.text} />
          </Action>
        </View>
        <KeyboardAvoidingView
          style={{ flex: 1 }}
          behavior={Platform.OS === "ios" ? "padding" : "height"}
        >
          {children}
        </KeyboardAvoidingView>
      </SafeAreaView>
    </Modal>
  );
}

export function MobileEditor({
  draft,
  onChange,
  onSave,
  saving,
  calendars,
  editable = true,
  onDirty,
}: {
  draft: EventInput;
  onChange: (event: EventInput) => void;
  onSave: (event: EventInput) => void;
  saving: boolean;
  calendars: { id: string; summary: string }[];
  editable?: boolean;
  onDirty: () => void;
}) {
  const [startValue, setStartValue] = useState(
    `${dateKey(new Date(draft.start))} ${clockTime(draft.start)}`,
  );
  const [endValue, setEndValue] = useState(
    `${dateKey(new Date(draft.end))} ${clockTime(draft.end)}`,
  );
  const [error, setError] = useState("");
  const submit = () => {
    try {
      const start = startValue.trim().split(/\s+/);
      const end = endValue.trim().split(/\s+/);
      const startInstant = localInstant(start[0], draft.allDay ? "00:00" : start[1]);
      const endInstant = localInstant(end[0], draft.allDay ? "00:00" : end[1]);

      if (!draft.title.trim()) {
        setError("Add an event title.");

        return;
      }

      if (Date.parse(endInstant) <= Date.parse(startInstant)) {
        setError("The end must be after the start.");

        return;
      }

      onSave({
        ...draft,
        title: draft.title.trim(),
        start: startInstant,
        end: endInstant,
      });
      setError("");
    } catch {
      setError("Use YYYY-MM-DD HH:mm for both dates.");
    }
  };

  return (
    <ScrollView
      keyboardShouldPersistTaps="handled"
      keyboardDismissMode="on-drag"
      contentContainerStyle={{ padding: 24, gap: 16 }}
    >
      <TextInput
        accessibilityLabel="Event title"
        placeholder="Add a title"
        placeholderTextColor={colors.muted}
        style={inputStyle}
        value={draft.title}
        editable={!saving && !draft.readOnly}
        onChangeText={(title) => onChange({ ...draft, title })}
      />
      <Label muted>Starts</Label>
      <DateField
        label="Start date and time"
        value={startValue}
        onChange={(value) => {
          onDirty();
          setStartValue(value);
        }}
        disabled={saving || draft.readOnly || !editable}
        allDay={draft.allDay}
      />
      <Label muted>{draft.allDay ? "Ends before (exclusive date)" : "Ends"}</Label>
      <DateField
        label="End date and time"
        value={endValue}
        onChange={(value) => {
          onDirty();
          setEndValue(value);
        }}
        disabled={saving || draft.readOnly || !editable}
        allDay={draft.allDay}
      />
      <View className="flex-row justify-between items-center">
        <Label>All-day</Label>
        <Switch
          disabled={saving || draft.readOnly}
          value={draft.allDay}
          onValueChange={(allDay) => {
            try {
              if (allDay) {
                const endDate = addDays(startValue.slice(0, 10), 1);

                setStartValue(`${startValue.slice(0, 10)} 00:00`);
                setEndValue(`${endDate} 00:00`);
              }

              onChange({ ...draft, allDay });
              setError("");
            } catch {
              setError("Use YYYY-MM-DD HH:mm for the start date.");
            }
          }}
        />
      </View>
      <Label muted>Calendar</Label>
      {calendars.map((calendar) => (
        <Pressable
          disabled={saving || draft.readOnly}
          key={calendar.id}
          onPress={() => onChange({ ...draft, calendarId: calendar.id })}
          style={{
            backgroundColor: draft.calendarId === calendar.id ? "#444" : "#292929",
            padding: 12,
            borderRadius: 8,
          }}
        >
          <Label>{calendar.summary}</Label>
        </Pressable>
      ))}
      <TextInput
        accessibilityLabel="Location"
        placeholder="Location"
        placeholderTextColor={colors.muted}
        style={inputStyle}
        value={draft.location}
        editable={!saving && !draft.readOnly}
        onChangeText={(location) => onChange({ ...draft, location })}
      />
      <TextInput
        accessibilityLabel="Notes"
        placeholder="Notes"
        placeholderTextColor={colors.muted}
        style={inputStyle}
        multiline
        value={draft.description}
        editable={!saving && !draft.readOnly}
        onChangeText={(description) => onChange({ ...draft, description })}
      />
      <Label muted>
        Times use Asia/Jakarta. All-day ends are exclusive. Recurring edits affect this
        occurrence.
      </Label>
      {!!error && <Label>{error}</Label>}
      {!draft.readOnly && (
        <Pressable
          disabled={saving || !editable}
          className="bg-neutral-100 rounded-xl p-4 items-center"
          onPress={submit}
        >
          <Text style={{ fontFamily: "Inter", color: "#191919" }}>
            {saving ? "Saving…" : editable ? "Save event" : "Offline · viewing only"}
          </Text>
        </Pressable>
      )}
    </ScrollView>
  );
}
