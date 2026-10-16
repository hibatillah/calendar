import "./global.css";
import { useState, type ReactNode } from "react";
import { ActivityIndicator, View } from "react-native";
import { SafeAreaProvider } from "react-native-safe-area-context";
import { useFonts } from "expo-font";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { colors } from "@calendar/design-tokens";
export default function App({ children }: { children: ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());
  const [fontsLoaded, fontError] = useFonts({
    Inter: require("./assets/fonts/Inter.ttf"),
  });
  if (!fontsLoaded && !fontError)
    return (
      <View style={{ flex: 1, backgroundColor: colors.background }}>
        <ActivityIndicator color={colors.text} />
      </View>
    );
  return (
    <SafeAreaProvider>
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    </SafeAreaProvider>
  );
}
