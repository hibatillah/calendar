export type MobilePreferences = {
  weekStart: 0 | 1;
  hiddenCalendars: string[];
  reminderMinutes: number | null;
};
export const defaultPreferences: MobilePreferences = {
  weekStart: 1,
  hiddenCalendars: [],
  reminderMinutes: null,
};
export async function readPreferences(): Promise<MobilePreferences> {
  const value = localStorage.getItem("calendar.preferences");
  return value
    ? { ...defaultPreferences, ...JSON.parse(value) }
    : defaultPreferences;
}
export async function writePreferences(value: MobilePreferences) {
  localStorage.setItem("calendar.preferences", JSON.stringify(value));
}
