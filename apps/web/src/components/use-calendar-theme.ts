import { useEffect, useState } from "react";

export type CalendarTheme = "system" | "light" | "dark";

export function useCalendarTheme() {
  const [theme, setTheme] = useState<CalendarTheme>(() => {
    try {
      const saved = localStorage.getItem("calendar:theme");

      if (saved === "light" || saved === "dark") return saved;
    } catch {
      /* Storage may be unavailable in private browser contexts. */
    }

    return "system";
  });

  useEffect(() => {
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved =
        theme === "system" ? (preference.matches ? "dark" : "light") : theme;

      document.documentElement.dataset.theme = resolved;
      document.documentElement.classList.toggle("dark", resolved === "dark");
    };

    apply();
    preference.addEventListener("change", apply);

    return () => {
      preference.removeEventListener("change", apply);
      delete document.documentElement.dataset.theme;
      document.documentElement.classList.add("dark");
    };
  }, [theme]);

  const selectTheme = (value: CalendarTheme) => {
    setTheme(value);

    try {
      localStorage.setItem("calendar:theme", value);
    } catch {
      /* Keep this session's selection. */
    }
  };

  return { theme, selectTheme };
}
