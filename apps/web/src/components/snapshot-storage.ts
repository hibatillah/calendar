import type { SnapshotStorage } from "@calendar/api/react";

export const storage: SnapshotStorage = {
  async read(key) {
    const value = localStorage.getItem(`calendar:snapshot:${key}`);

    return value ? JSON.parse(value) : undefined;
  },
  async write(key, value) {
    localStorage.setItem(`calendar:snapshot:${key}`, JSON.stringify(value));
  },
  async clear() {
    Object.keys(localStorage)
      .filter((key) => key.startsWith("calendar:snapshot:"))
      .forEach((key) => localStorage.removeItem(key));
  },
};
