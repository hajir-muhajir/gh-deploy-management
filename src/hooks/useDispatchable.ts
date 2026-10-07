import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, DispatchWorkflow } from "../lib/github";

export interface UseDispatchable {
  workflows: DispatchWorkflow[];
  loading: boolean;
  error: ApiError | null;
  reload: () => void;
}

/**
 * The manually startable workflows of one repository.
 *
 * Unlike the other data hooks this one is deliberately lazy: answering it costs
 * one request per active workflow — nine for this account's busiest repo —
 * because neither the trigger nor the input definitions exist anywhere in REST.
 * Callers pass `enabled` only while the Run tab is open.
 */
export function useDispatchable(fullName: string | null, enabled: boolean): UseDispatchable {
  const [workflows, setWorkflows] = useState<DispatchWorkflow[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!enabled || !fullName) {
      // Leave `workflows` alone: closing the tab should not throw away a list
      // that cost nine requests, and the repo guard below keeps it honest.
      return;
    }
    const [owner, name] = fullName.split("/");
    let alive = true;
    setLoading(true);
    api
      .listDispatchable(owner, name)
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

  // Switching repository must not leave the previous repo's workflows on
  // screen, which is a real risk given the fetch above is skipped while the
  // tab is closed.
  useEffect(() => {
    setWorkflows([]);
    setError(null);
  }, [fullName]);

  const reload = useCallback(() => setNonce((n) => n + 1), []);

  return { workflows, loading, error, reload };
}
