import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, PullRequestInfo } from "../lib/github";

export interface UsePulls {
  pulls: PullRequestInfo[];
  /** More open pull requests exist than the page on screen. */
  hasMore: boolean;
  loading: boolean;
  error: ApiError | null;
  refresh: () => void;
}

/**
 * Open pull requests for one repository.
 *
 * Not lazy like `useDispatchable`, even though it also costs one request per
 * item: the tray badge counts review requests, so this has to be loaded
 * whichever tab is on screen.
 */
export function usePulls(fullName: string | null, enabled: boolean): UsePulls {
  const [pulls, setPulls] = useState<PullRequestInfo[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!enabled || !fullName) {
      setPulls([]);
      setHasMore(false);
      setError(null);
      return;
    }
    const [owner, name] = fullName.split("/");
    let alive = true;
    setLoading(true);
    api
      .listPulls(owner, name)
      .then((page) => {
        if (!alive) return;
        setPulls(page.pulls);
        setHasMore(page.hasMore);
        setError(null);
      })
      .catch((caught) => alive && setError(api.toApiError(caught)))
      .finally(() => alive && setLoading(false));
    return () => {
      alive = false;
    };
  }, [enabled, fullName, nonce]);

  const refresh = useCallback(() => setNonce((n) => n + 1), []);

  return { pulls, hasMore, loading, error, refresh };
}
