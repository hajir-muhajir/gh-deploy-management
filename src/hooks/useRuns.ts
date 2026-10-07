import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, RunInfo } from "../lib/github";

export interface UseRuns {
  runs: RunInfo[];
  loading: boolean;
  error: ApiError | null;
  refresh: () => void;
  /** Cancel a run, then refetch so the list reflects the new state. */
  cancel: (runId: number) => Promise<void>;
  rerun: (runId: number) => Promise<void>;
}

/** Recent workflow runs for one repository. */
export function useRuns(fullName: string | null, enabled: boolean): UseRuns {
  const [runs, setRuns] = useState<RunInfo[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!enabled || !fullName) {
      setRuns([]);
      setError(null);
      return;
    }
    const [owner, name] = fullName.split("/");
    let alive = true;
    setLoading(true);
    api
      .listRuns(owner, name)
      .then((list) => {
        if (!alive) return;
        setRuns(list);
        setError(null);
      })
      .catch((caught) => alive && setError(api.toApiError(caught)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [enabled, fullName, nonce]);

  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  /**
   * GitHub answers 202 Accepted before the run has actually changed state, so
   * the refetch below may still show the old status for a moment. The caller
   * surfaces any error (most likely a missing Actions: write permission).
   */
  const act = useCallback(
    async (runId: number, send: (owner: string, name: string, id: number) => Promise<void>) => {
      if (!fullName) return;
      const [owner, name] = fullName.split("/");
      await send(owner, name, runId);
      setNonce((n) => n + 1);
    },
    [fullName],
  );

  const cancel = useCallback((runId: number) => act(runId, api.cancelRun), [act]);
  const rerun = useCallback((runId: number) => act(runId, api.rerunRun), [act]);

  return { runs, loading, error, refresh, cancel, rerun };
}
