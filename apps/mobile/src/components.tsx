import { colors } from "@calendar/design-tokens";
import { X } from "lucide-react-native";
import {
  KeyboardAvoidingView,
  Modal,
  Platform,
  Pressable,
  Text,
  View,
} from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

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
