import { openDatabaseAsync } from "expo-sqlite";
import type { SnapshotStorage } from "@calendar/api/react";

const database = openDatabaseAsync("calendar-preview.db").then(async (database) => {
  await database.execAsync(
    "CREATE TABLE IF NOT EXISTS snapshots (cache_key TEXT PRIMARY KEY, payload TEXT NOT NULL)",
  );

  return database;
});
export const snapshotStorage: SnapshotStorage = {
  async read(key) {
    const row = await (
      await database
    ).getFirstAsync<{ payload: string }>(
      "SELECT payload FROM snapshots WHERE cache_key = ?",
      key,
    );

    return row ? JSON.parse(row.payload) : undefined;
  },
  async write(key, value) {
    await (
      await database
    ).runAsync(
      "INSERT OR REPLACE INTO snapshots (cache_key, payload) VALUES (?, ?)",
      key,
      JSON.stringify(value),
    );
  },
  async clear() {
    await (await database).execAsync("DELETE FROM snapshots");
  },
};
