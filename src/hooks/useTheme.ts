import { useCallback, useEffect, useState } from "react";
import { persist, readEnum } from "../lib/prefs";
import type { AccentId, ResolvedTheme, ThemePreference } from "../types";

const THEME_KEY = "ghdm.theme";
const ACCENT_KEY = "ghdm.accent";

const THEME_VALUES: ThemePreference[] = ["system", "light", "dark"];
const ACCENT_VALUES: AccentId[] = ["blue", "indigo", "graphite"];

export function useTheme() {
  const [preference, setPreferenceState] = useState<ThemePreference>(() =>
    readEnum(THEME_KEY, THEME_VALUES, "system"),
  );
  const [accent, setAccentState] = useState<AccentId>(() =>
    readEnum(ACCENT_KEY, ACCENT_VALUES, "blue"),
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
