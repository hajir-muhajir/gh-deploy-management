import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "../lib/github";
import type { ApiError, TokenInfo } from "../lib/github";

export type AuthStatus =
  /** Reading the stored token on startup. */
  | "loading"
  /** No token stored. */
  | "disconnected"
  /** Token stored and accepted by GitHub. */
  | "connected"
  /** GitHub rejected the token (401). */
  | "invalid"
  /** Network / rate limit / credential store failure — says nothing about the token. */
  | "error";

export interface GitHubAuth {
  status: AuthStatus;
  info: TokenInfo | null;
  error: ApiError | null;
  /** True while a Save round-trip is in flight. */
  saving: boolean;
  /**
   * Validates then stores. An empty string clears the token.
   * Resolves to false when GitHub rejected it, so the caller can keep the draft.
   */
  save: (token: string, repoForActions?: string) => Promise<boolean>;
  /** Re-probes the Actions permission now that a repository is known. */
  recheck: (repoForActions: string) => Promise<void>;
}

function statusFor(error: ApiError): AuthStatus {
  return error.kind === "invalidToken" ? "invalid" : "error";
}

export function useGitHubAuth(): GitHubAuth {
  const [status, setStatus] = useState<AuthStatus>("loading");
  const [info, setInfo] = useState<TokenInfo | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [saving, setSaving] = useState(false);
  const alive = useRef(true);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);

  const apply = useCallback((next: TokenInfo | null) => {
    if (!alive.current) return;
    setInfo(next);
    setError(null);
    setStatus(next ? "connected" : "disconnected");
  }, []);

  const fail = useCallback((caught: unknown) => {
    if (!alive.current) return;
    const apiError = api.toApiError(caught);
    setError(apiError);
    // A rejected token is no longer usable, so drop the stale identity. Other
    // failures (network, rate limit) say nothing about the token, so keep it.
    if (apiError.kind === "invalidToken") setInfo(null);
    setStatus(statusFor(apiError));
  }, []);

  useEffect(() => {
    api.tokenInfo().then(apply).catch(fail);
  }, [apply, fail]);

  const save = useCallback(
    async (token: string, repoForActions?: string) => {
      setSaving(true);
      try {
        apply(await api.saveToken(token, repoForActions));
        return true;
      } catch (caught) {
        fail(caught);
        return false;
      } finally {
        if (alive.current) setSaving(false);
      }
    },
    [apply, fail],
  );

  const recheck = useCallback(async (repoForActions: string) => {
    try {
      const next = await api.tokenInfo(repoForActions);
      if (alive.current && next) setInfo(next);
    } catch {
      // A failed re-probe must not knock out an otherwise working session.
    }
  }, []);

  return { status, info, error, saving, save, recheck };
}
