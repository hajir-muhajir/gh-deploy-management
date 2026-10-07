import { useCallback, useEffect, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, ReleaseInfo } from "../lib/github";

export interface UseReleases {
  releases: ReleaseInfo[];
  /** More releases exist than the page on screen. */
  hasMore: boolean;
  loading: boolean;
  error: ApiError | null;
  refresh: () => void;
}

/** The most recent page of releases for one repository. */
export function useReleases(fullName: string | null, enabled: boolean): UseReleases {
  const [releases, setReleases] = useState<ReleaseInfo[]>([]);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<ApiError | null>(null);
  const [nonce, setNonce] = useState(0);

  useEffect(() => {
    if (!enabled || !fullName) {
      setReleases([]);
      setHasMore(false);
      setError(null);
      return;
    }
    const [owner, name] = fullName.split("/");
    let alive = true;
    setLoading(true);
    api
      .listReleases(owner, name)
      .then((page) => {
        if (!alive) return;
        setReleases(page.releases);
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

  return { releases, hasMore, loading, error, refresh };
}
