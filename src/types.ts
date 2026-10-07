export type RunStatus = "success" | "failure" | "running" | "queued" | "cancelled";

export type CheckStatus = "success" | "failure" | "pending";

export interface Repo {
  owner: string;
  name: string;
  /** Two-letter initials shown in the avatar tile. */
  ini: string;
  /** [major, minor, patch] used to generate dummy releases. */
  ver: [number, number, number];
  visible: boolean;
}

export interface Release {
  tag: string;
  name: string;
  when: string;
  author: string;
  latest?: boolean;
  pre?: boolean;
  notes: string[];
}

export interface WorkflowRun {
  wf: string;
  title: string;
  branch: string;
  n: number;
  status: RunStatus;
  pct?: number;
  ago: string;
  dur: string;
}

export type WorkflowInput =
  | { key: string; label: string; type: "text"; def: string; ph?: string; hint?: string }
  | { key: string; label: string; type: "bool"; def: boolean; hint?: string }
  | { key: string; label: string; type: "choice"; def: string; options: string[]; hint?: string };

export interface Workflow {
  name: string;
  file: string;
  inputs: WorkflowInput[];
}

export interface PullRequest {
  n: number;
  title: string;
  author: string;
  head: string;
  draft: boolean;
  review: boolean;
  checks: CheckStatus;
  updated: string;
}

export type TabId = "releases" | "run" | "actions" | "prs";
export type ViewId = TabId | "settings";
export type PrFilterId = "open" | "draft" | "review";

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type AccentId = "blue" | "indigo" | "graphite";

export type InputValue = string | boolean;
