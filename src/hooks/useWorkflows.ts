import { useCallback, useEffect, useMemo, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, WorkflowInfo } from "../lib/github";

const VISIBILITY_KEY = "ghdm.workflows.visibility";

export interface WorkflowEntry extends WorkflowInfo {
  /** GitHub state is "active"; anything else means it will not run again. */
  active: boolean;
  visible: boolean;
}

export interface UseWorkflows {
  workflows: WorkflowEntry[];
  loading: boolean;
  error: ApiError | null;
  /** Workflow ids whose runs the Actions tab should leave out. */
  hiddenWorkflowIds: Set<number>;
  toggle: (path: string) => void;
  reload: () => void;
}

/**
 * `{ "owner/repo": { ".github/workflows/deploy.yml": true } }`
 *
 * Keyed by `path`: workflow *names* are not unique — a repo can genuinely have
 * two workflows called "Deploy Production" — and ids change when a workflow is
 * deleted and recreated.
 *
 * An entry is an explicit override. Its absence means "use the default", which
 * is visible for active workflows and hidden for disabled ones. A plain list of
 * hidden paths could not express "this disabled workflow should be shown".
 */
type VisibilityMap = Record<string, Record<string, boolean>>;

function readVisibility(): VisibilityMap {
  try {
    const raw = localStorage.getItem(VISIBILITY_KEY);
    const parsed: unknown = raw ? JSON.parse(raw) : {};
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};

    const result: VisibilityMap = {};
    for (const [repo, overrides] of Object.entries(parsed as Record<string, unknown>)) {
      if (!overrides || typeof overrides !== "object" || Array.isArray(overrides)) continue;
      const clean: Record<string, boolean> = {};
      for (const [path, value] of Object.entries(overrides as Record<string, unknown>)) {
        if (typeof value === "boolean") clean[path] = value;
      }
      result[repo] = clean;
    }
    return result;
  } catch {
    return {};
  }
}

function writeVisibility(value: VisibilityMap) {
  try {
    localStorage.setItem(VISIBILITY_KEY, JSON.stringify(value));
  } catch {
    /* storage unavailable */
  }
}

/** Workflows of one repository plus which of them the user wants to see. */
export function useWorkflows(fullName: string | null, enabled: boolean): UseWorkflows {
  const [workflows, setWorkflows] = useState<WorkflowInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [visibility, setVisibility] = useState<VisibilityMap>(() => readVisibility());
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!enabled || !fullName) {
      setWorkflows([]);
      setError(null);
      return;
    }
    const [owner, name] = fullName.split("/");
    let alive = true;
    setLoading(true);
    api
      .listWorkflows(owner, name)
      .then((list) => {
        if (!alive) return;
        setWorkflows(list);
        setError(null);
      })
      .catch((caught) => alive && setError(api.toApiError(caught)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [enabled, fullName, nonce]);

  const overrides = useMemo(
    () => (fullName ? (visibility[fullName] ?? {}) : {}),
    [visibility, fullName],
  );

  const entries: WorkflowEntry[] = useMemo(
    () =>
      workflows.map((workflow) => {
        const active = workflow.state === "active";
        return { ...workflow, active, visible: overrides[workflow.path] ?? active };
      }),
    [workflows, overrides],
  );

  const hiddenWorkflowIds = useMemo(
    () => new Set(entries.filter((entry) => !entry.visible).map((entry) => entry.id)),
    [entries],
  );

  const toggle = useCallback(
    (path: string) => {
      if (!fullName) return;
      const entry = entries.find((candidate) => candidate.path === path);
      if (!entry) return;

      setVisibility((prev) => {
        const next: VisibilityMap = {
          ...prev,
          [fullName]: { ...(prev[fullName] ?? {}), [path]: !entry.visible },
        };
        writeVisibility(next);
        return next;
      });
    },
    [entries, fullName],
  );

  const reload = useCallback(() => setNonce((n) => n + 1), []);

  return { workflows: entries, loading, error, hiddenWorkflowIds, toggle, reload };
}
