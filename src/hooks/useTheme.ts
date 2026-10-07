import { useCallback, useEffect, useState } from "react";
import type { AccentId, ResolvedTheme, ThemePreference } from "../types";

const THEME_KEY = "ghdm.theme";
const ACCENT_KEY = "ghdm.accent";

const THEME_VALUES: ThemePreference[] = ["system", "light", "dark"];
const ACCENT_VALUES: AccentId[] = ["blue", "indigo", "graphite"];

function read<T extends string>(key: string, allowed: T[], fallback: T): T {
  try {
    const stored = localStorage.getItem(key) as T | null;
    if (stored && allowed.includes(stored)) return stored;
  } catch {
    /* localStorage unavailable (private mode / embedded webview) */
  }
  return fallback;
}

function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* ignore */
  }
}

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    read(THEME_KEY, THEME_VALUES, "system"),
  );
  const [accent, setAccentState] = useState<AccentId>(() =>
    read(ACCENT_KEY, ACCENT_VALUES, "blue"),
  );
  const [systemDark, setSystemDark] = useState(
    () => typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches,
  );

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = (e: MediaQueryListEvent) => setSystemDark(e.matches);
    mq.addEventListener("change", onChange);
    return () => mq.removeEventListener("change", onChange);
  }, []);

  const setPreference = useCallback((value: ThemePreference) => {
    setPreferenceState(value);
    persist(THEME_KEY, value);
  }, []);

  const setAccent = useCallback((value: AccentId) => {
    setAccentState(value);
    persist(ACCENT_KEY, value);
  }, []);

  const theme: ResolvedTheme =
    preference === "system" ? (systemDark ? "dark" : "light") : preference;

  return { theme, preference, setPreference, accent, setAccent };
}
