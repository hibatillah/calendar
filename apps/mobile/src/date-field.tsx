import { TextInput } from "react-native";
import { localInstant } from "@calendar/calendar";

export type DateFieldProps = {
  label: string;
  value: string;
  onChange: (value: string) => void;
  disabled: boolean;
  allDay: boolean;
};
export function parseDateField(value: string) {
  const [date, time] = value.trim().split(/\s+/);
  return new Date(localInstant(date, time || "00:00"));
}
export function DateField({
  label,
  value,
  onChange,
  disabled,
}: DateFieldProps) {
  return (
    <TextInput
      accessibilityLabel={label}
      value={value}
      onChangeText={onChange}
      editable={!disabled}
      style={{
        color: "#e9e9e9",
        fontFamily: "Inter",
        padding: 12,
        borderRadius: 8,
        backgroundColor: "#303030",
      }}
    />
  );
}
