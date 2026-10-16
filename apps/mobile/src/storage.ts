import type { SnapshotStorage } from "@calendar/api/react";

export const snapshotStorage: SnapshotStorage = {
  async read(key) {
    const value = localStorage.getItem(`calendar:mobile:${key}`);

    return value ? JSON.parse(value) : undefined;
  },
  async write(key, value) {
    localStorage.setItem(`calendar:mobile:${key}`, JSON.stringify(value));
  },
  async clear() {
    Object.keys(localStorage)
      .filter((key) => key.startsWith("calendar:mobile:"))
      .forEach((key) => localStorage.removeItem(key));
  },
};
