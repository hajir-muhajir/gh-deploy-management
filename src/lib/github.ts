import { invoke } from "@tauri-apps/api/core";

/** Mirrors the Rust `ApiError` kinds in src-tauri/src/github.rs. */
export type ApiErrorKind =
  | "invalidToken"
  | "forbidden"
  | "notFound"
  | "rateLimited"
  | "network"
  | "keyring"
  | "notConnected"
  /** Running outside the Tauri webview, where no command bridge exists. */
  | "unavailable"
  | "unknown";

export interface ApiError {
  kind: ApiErrorKind;
  message: string;
}

export type TokenKind = "classic" | "fineGrained";

export interface RateLimit {
  limit: number;
  remaining: number;
  /** Unix seconds. */
  reset: number;
}

export interface TokenInfo {
  login: string;
  avatarUrl: string;
  kind: TokenKind;
  /** Always empty for fine-grained tokens — their permissions are not enumerable. */
  scopes: string[];
  /** Classic tokens only: required scopes that are absent. */
  missingScopes: string[];
  reposVerified: boolean;
  /** False until a concrete repository has been probed. */
  actionsVerified: boolean;
  rate: RateLimit;
}

export interface RepoInfo {
  owner: string;
  name: string;
  fullName: string;
  defaultBranch: string;
  private: boolean;
  archived: boolean;
  canPush: boolean;
}

/** Normalised by Rust; never a raw GitHub status. */
export type RunStatus = "success" | "failure" | "running" | "queued" | "cancelled";

export interface WorkflowInfo {
  id: number;
  name: string;
  /** Unique within a repository, unlike `name`. This is the storage key. */
  path: string;
  /** active | disabled_manually | disabled_inactivity | ... */
  state: string;
  htmlUrl: string;
}

export interface RunInfo {
  id: number;
  workflowId: number;
  workflowName: string;
  title: string;
  runNumber: number;
  branch: string;
  status: RunStatus;
  actor: string;
  event: string;
  htmlUrl: string;
  /** ISO timestamps; formatting happens in lib/time.ts. */
  startedAt: string;
  updatedAt: string;
}

export interface ReleaseInfo {
  id: number;
  tag: string;
  name: string;
  /** Markdown release notes, straight from GitHub. May be empty. */
  body: string;
  draft: boolean;
  prerelease: boolean;
  /** Derived in Rust: the newest non-draft, non-prerelease release. */
  latest: boolean;
  author: string;
  publishedAt: string;
  htmlUrl: string;
}

export interface ReleasesPage {
  releases: ReleaseInfo[];
  /** GitHub reported further pages, so the list on screen is partial. */
  hasMore: boolean;
}

export interface ReposPage {
  repos: RepoInfo[];
  truncated: boolean;
}

/**
 * `invoke` rejects with whatever the command's error type serialised to, so a
 * typed `ApiError` arrives as a plain object. Anything else (a panic, a missing
 * command) arrives as a string and is normalised here.
 */
export function toApiError(error: unknown): ApiError {
  if (error && typeof error === "object" && "kind" in error && "message" in error) {
    return error as ApiError;
  }
  return { kind: "unknown", message: String(error) };
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  // In a plain browser `invoke` blows up with an opaque TypeError, which would
  // surface verbatim in the UI. Fail with something meaningful instead.
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window)) {
    throw { kind: "unavailable", message: "GitHub access is only available in the desktop app" } satisfies ApiError;
  }
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toApiError(error);
  }
}

/**
 * Validates the token against GitHub and stores it only if valid, so a failed
 * save leaves a previously working token untouched. An empty string clears it.
 *
 * @param repoForActions `owner/name` used to probe the Actions permission of a
 *   fine-grained token, which GitHub only exposes per repository.
 */
export function saveToken(token: string, repoForActions?: string) {
  return call<TokenInfo | null>("github_save_token", { token, repoForActions });
}

/** Re-validates the stored token on startup so a revoked one is caught. */
export function tokenInfo(repoForActions?: string) {
  return call<TokenInfo | null>("github_token_info", { repoForActions });
}

export function clearToken() {
  return call<void>("github_clear_token");
}

export function listRepos() {
  return call<ReposPage>("github_list_repos");
}

export function addRepo(owner: string, name: string) {
  return call<RepoInfo>("github_add_repo", { owner, name });
}

export function probeActions(owner: string, name: string) {
  return call<boolean>("github_probe_actions", { owner, name });
}

export function listWorkflows(owner: string, name: string) {
  return call<WorkflowInfo[]>("github_list_workflows", { owner, name });
}

export function listRuns(owner: string, name: string) {
  return call<RunInfo[]>("github_list_runs", { owner, name });
}

export function listReleases(owner: string, name: string) {
  return call<ReleasesPage>("github_list_releases", { owner, name });
}

/** Needs the Actions *write* permission; rejects with kind "forbidden" without it. */
export function cancelRun(owner: string, name: string, runId: number) {
  return call<void>("github_cancel_run", { owner, name, runId });
}

export function rerunRun(owner: string, name: string, runId: number) {
  return call<void>("github_rerun_run", { owner, name, runId });
}
