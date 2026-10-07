import { useCallback, useEffect, useMemo, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, RepoInfo } from "../lib/github";
import type { Repo } from "../types";

const TRACKED_KEY = "ghdm.repos.tracked";
const HIDDEN_KEY = "ghdm.repos.hidden";

export interface UseRepos {
  /** Every repository the token can reach — the search corpus, never rendered whole. */
  available: RepoInfo[];
  /** Repositories the user picked; this is what Settings and the switcher show. */
  tracked: Repo[];
  loading: boolean;
  error: ApiError | null;
  /** True when the account has more repos than the fetch cap. */
  truncated: boolean;
  /** Adds by full name; falls back to a direct lookup for repos outside `available`. */
  add: (fullName: string) => Promise<void>;
  remove: (fullName: string) => void;
  toggleVisible: (fullName: string) => void;
  reload: () => void;
}

function readList(key: string): string[] {
  try {
    const raw = localStorage.getItem(key);
    const parsed: unknown = raw ? JSON.parse(raw) : [];
    return Array.isArray(parsed) ? parsed.filter((v): v is string => typeof v === "string") : [];
  } catch {
    return [];
  }
}

function writeList(key: string, value: string[]) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* storage unavailable */
  }
}

/** Two-letter tile label, e.g. "api-gateway" -> "AG". Falls back to the first two letters. */
export function initialsOf(name: string): string {
  const words = name.split(/[^a-z0-9]+/i).filter(Boolean);
  if (words.length >= 2) return (words[0][0] + words[1][0]).toUpperCase();
  return name.replace(/[^a-z0-9]/gi, "").slice(0, 2).toUpperCase() || "??";
}

function toRepo(info: RepoInfo, visible: boolean): Repo {
  return {
    owner: info.owner,
    name: info.name,
    fullName: info.fullName,
    ini: initialsOf(info.name),
    defaultBranch: info.defaultBranch,
    private: info.private,
    archived: info.archived,
    canPush: info.canPush,
    visible,
  };
}

const keyOf = (fullName: string) => fullName.toLowerCase();

export function useRepos(connected: boolean): UseRepos {
  const [available, setAvailable] = useState<RepoInfo[]>([]);
  const [extra, setExtra] = useState<RepoInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [truncated, setTruncated] = useState(false);

  const [trackedNames, setTrackedNames] = useState<string[]>(() => readList(TRACKED_KEY));
  const [hiddenNames, setHiddenNames] = useState<string[]>(() => readList(HIDDEN_KEY));
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!connected) {
      setAvailable([]);
      setError(null);
      return;
    }
    let alive = true;
    setLoading(true);
    api
      .listRepos()
      .then((page) => {
        if (!alive) return;
        setAvailable(page.repos);
        setTruncated(page.truncated);
        setError(null);
      })
      .catch((caught) => alive && setError(api.toApiError(caught)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [connected, nonce]);

  /** Lookup across both the listing and repos fetched individually. */
  const byKey = useMemo(() => {
    const map = new Map<string, RepoInfo>();
    for (const info of [...available, ...extra]) map.set(keyOf(info.fullName), info);
    return map;
  }, [available, extra]);

  const hidden = useMemo(() => new Set(hiddenNames.map(keyOf)), [hiddenNames]);

  // A tracked name with no matching RepoInfo yet (listing still loading, or the
  // repo lost access) is skipped rather than rendered as a broken row.
  const tracked = useMemo(
    () =>
      trackedNames
        .map((fullName) => byKey.get(keyOf(fullName)))
        .filter((info): info is RepoInfo => Boolean(info))
        .map((info) => toRepo(info, !hidden.has(keyOf(info.fullName)))),
    [trackedNames, byKey, hidden],
  );

  const persistTracked = useCallback((next: string[]) => {
    setTrackedNames(next);
    writeList(TRACKED_KEY, next);
  }, []);

  const add = useCallback(
    async (fullName: string) => {
      const trimmed = fullName.trim();
      const key = keyOf(trimmed);
      if (trackedNames.some((n) => keyOf(n) === key)) {
        throw { kind: "unknown", message: "Already in the list" } satisfies ApiError;
      }

      let info = byKey.get(key);
      if (!info) {
        // Not in /user/repos — could be a public repo elsewhere. Ask directly.
        const [owner, name] = trimmed.split("/");
        if (!owner || !name) {
          throw { kind: "unknown", message: "Use owner/repository" } satisfies ApiError;
        }
        info = await api.addRepo(owner, name);
        setExtra((prev) => [...prev, info as RepoInfo]);
      }

      persistTracked([...trackedNames, info.fullName]);
    },
    [byKey, trackedNames, persistTracked],
  );

  const remove = useCallback(
    (fullName: string) => {
      persistTracked(trackedNames.filter((n) => keyOf(n) !== keyOf(fullName)));
    },
    [trackedNames, persistTracked],
  );

  const toggleVisible = useCallback(
    (fullName: string) => {
      const key = keyOf(fullName);
      const isHidden = hidden.has(key);
      // Keep at least one repo visible, otherwise the switcher has nothing to show.
      if (!isHidden && tracked.filter((r) => r.visible).length <= 1) return;
      const next = isHidden
        ? hiddenNames.filter((n) => keyOf(n) !== key)
        : [...hiddenNames, fullName];
      setHiddenNames(next);
      writeList(HIDDEN_KEY, next);
    },
    [hidden, hiddenNames, tracked],
  );

  const reload = useCallback(() => setNonce((n) => n + 1), []);

  return { available, tracked, loading, error, truncated, add, remove, toggleVisible, reload };
}
