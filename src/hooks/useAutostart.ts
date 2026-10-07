import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/github";
import { persist, readEnum } from "../lib/prefs";

/**
 * Marks that the first-run default has already been applied. Without it the app
 * would re-register itself on every launch and silently undo the user turning
 * the toggle off.
 */
const DECIDED_KEY = "ghdm.autostart.decided";

export interface UseAutostart {
  enabled: boolean;
  setEnabled: (value: boolean) => void;
}

/**
 * Whether the app runs at Windows sign-in.
 *
 * The registry entry is the source of truth, so the toggle reflects what
 * Windows will actually do rather than what we last asked for. Enabled by
 * default on first run, and never re-enabled afterwards.
 */
export function useAutostart(enabled: boolean): UseAutostart {
  const [value, setValue] = useState(false);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  useEffect(() => {
    if (!enabled) return;
    void (async () => {
      try {
        let current = await api.autostartEnabled();
        // First run: opt in on the user's behalf, once.
        if (!current && readEnum(DECIDED_KEY, ["yes", "no"] as const, "no") === "no") {
          current = await api.setAutostart(true);
        }
        persist(DECIDED_KEY, "yes");
        if (!alive.current) return;
        setValue(current);
      } catch {
        // Outside Tauri there is no registry to read, and a failure here must
        // not take the Settings tab down with it.
      }
    })();
  }, [enabled]);

  const setEnabled = useCallback((next: boolean) => {
    // Optimistic: the switch has to feel immediate, and the registry write is
    // reconciled a moment later.
    setValue(next);
    void api
      .setAutostart(next)
      .then((actual) => alive.current && setValue(actual))
      .catch(() => alive.current && setValue(!next));
  }, []);

  return { enabled: value, setEnabled };
}
