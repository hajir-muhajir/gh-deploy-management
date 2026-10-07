import { useCallback, useEffect, useRef, useState } from "react";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { persist } from "../lib/prefs";
import { useIsTauri } from "./useIsTauri";

const DISMISS_KEY = "ghdm.update.dismissed";
const SNOOZE_MS = 24 * 60 * 60 * 1000;

export type UpdateStatus = "idle" | "available" | "downloading" | "error";

export interface UseUpdater {
  /** The offered version, or null when there is nothing to show. */
  version: string | null;
  status: UpdateStatus;
  /** Downloads, installs and restarts into the new version. */
  install: () => void;
  /** Hides the banner for this version for 24 hours. */
  dismiss: () => void;
}

/**
 * True while the user's "not now" on this exact version is still fresh.
 *
 * The version is part of the stored value so dismissing 0.1.1 does not also
 * silence 0.1.2 released the next day. Anything unparseable counts as not
 * dismissed — a corrupt entry must not hide updates forever.
 */
function isSnoozed(version: string, now: number): boolean {
  try {
    const raw = localStorage.getItem(DISMISS_KEY);
    if (!raw) return false;
    const separator = raw.lastIndexOf(":");
    if (separator < 0) return false;
    const at = Number(raw.slice(separator + 1));
    if (!Number.isFinite(at)) return false;
    return raw.slice(0, separator) === version && now - at < SNOOZE_MS;
  } catch {
    /* storage unavailable (private mode / embedded webview) */
    return false;
  }
}

/**
 * Checks GitHub for a newer release once per launch.
 *
 * Every failure is swallowed into `idle`: being offline, a rate limit or a
 * missing latest.json are all normal, and none of them should surface to a
 * user who never asked about updates.
 */
export function useUpdater(): UseUpdater {
  const isTauri = useIsTauri();

  const [version, setVersion] = useState<string | null>(null);
  const [status, setStatus] = useState<UpdateStatus>("idle");
  const update = useRef<Update | null>(null);
  // StrictMode double-invokes effects in dev; the check costs a request.
  const checked = useRef(false);

  useEffect(() => {
    if (!isTauri || checked.current) return;
    checked.current = true;

    void (async () => {
      try {
        const found = await check();
        if (!found || isSnoozed(found.version, Date.now())) return;
        update.current = found;
        setVersion(found.version);
        setStatus("available");
      } catch {
        /* no update today */
      }
    })();
  }, [isTauri]);

  const install = useCallback(() => {
    const found = update.current;
    if (!found) return;
    setStatus("downloading");
    void (async () => {
      try {
        await found.downloadAndInstall();
        await relaunch();
      } catch {
        setStatus("error");
      }
    })();
  }, []);

  const dismiss = useCallback(() => {
    if (version) persist(DISMISS_KEY, `${version}:${Date.now()}`);
    setStatus("idle");
    setVersion(null);
  }, [version]);

  return { version, status, install, dismiss };
}
