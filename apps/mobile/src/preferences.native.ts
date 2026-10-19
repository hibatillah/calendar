import { openDatabaseAsync } from "expo-sqlite";
import type { MobilePreferences } from "./preferences";
export const defaultPreferences: MobilePreferences = {
  weekStart: 1,
  hiddenCalendars: [],
  reminderMinutes: null,
};
const database = openDatabaseAsync("calendar-settings.db").then(
  async (database) => {
    await database.execAsync(
      "CREATE TABLE IF NOT EXISTS preferences (id INTEGER PRIMARY KEY, payload TEXT NOT NULL)",
    );
    return database;
  },
);
export async function readPreferences(): Promise<MobilePreferences> {
  const row = await (
    await database
  ).getFirstAsync<{ payload: string }>(
    "SELECT payload FROM preferences WHERE id = 1",
  );
  return row
    ? { ...defaultPreferences, ...JSON.parse(row.payload) }
    : defaultPreferences;
}
export async function writePreferences(value: MobilePreferences) {
  await (
    await database
  ).runAsync(
    "INSERT OR REPLACE INTO preferences (id, payload) VALUES (1, ?)",
    JSON.stringify(value),
  );
}
