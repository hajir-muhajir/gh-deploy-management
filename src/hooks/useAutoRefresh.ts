import { useCallback, useState } from "react";
import { persist, readEnum } from "../lib/prefs";
import type { RefreshInterval } from "../types";

const KEY = "ghdm.refresh.interval";

export const REFRESH_INTERVALS: RefreshInterval[] = ["off", "1m", "5m", "15m"];

/**
 * Milliseconds between ticks, or null when polling is off.
 *
 * A tick costs one request for the runs plus one per open pull request, so a
 * busy repository at "1m" is a meaningful slice of the 5000/hour budget — hence
 * the 5m default rather than the fastest option.
 */
export function intervalMs(interval: RefreshInterval): number | null {
  switch (interval) {
    case "1m":
      return 60_000;
    case "5m":
      return 5 * 60_000;
    case "15m":
      return 15 * 60_000;
    case "off":
      return null;
  }
}

export interface UseAutoRefresh {
  interval: RefreshInterval;
  setInterval: (value: RefreshInterval) => void;
}

export function useAutoRefresh(): UseAutoRefresh {
  const [interval, setIntervalState] = useState<RefreshInterval>(() =>
    readEnum(KEY, REFRESH_INTERVALS, "5m"),
  );

  const setInterval = useCallback((value: RefreshInterval) => {
    setIntervalState(value);
    persist(KEY, value);
  }, []);

  return { interval, setInterval };
}
