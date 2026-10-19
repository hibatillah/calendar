import { Platform } from "react-native";
import * as SecureStore from "expo-secure-store";
export type ConnectionSettings = { endpoint: string; token: string };
const key = "calendar.local.connection";
export async function loadConnection(): Promise<ConnectionSettings | null> {
  const value =
    Platform.OS === "web"
      ? sessionStorage.getItem(key)
      : await SecureStore.getItemAsync(key);
  return value ? JSON.parse(value) : null;
}
export async function saveConnection(value: ConnectionSettings) {
  const endpoint = new URL(value.endpoint);
  if (
    endpoint.protocol !== "https:" &&
    !(
      endpoint.protocol === "http:" &&
      ["127.0.0.1", "localhost", "10.0.2.2"].includes(endpoint.hostname)
    )
  )
    throw new Error("Use HTTPS or a loopback development connection.");
  if (Platform.OS === "web") sessionStorage.setItem(key, JSON.stringify(value));
  else await SecureStore.setItemAsync(key, JSON.stringify(value));
}
export async function clearConnection() {
  if (Platform.OS === "web") sessionStorage.removeItem(key);
  else await SecureStore.deleteItemAsync(key);
}
