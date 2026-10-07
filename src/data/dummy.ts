import type { CheckStatus, InputValue, PullRequest, Workflow } from "../types";

export const WORKFLOWS: Workflow[] = [
  {
    name: "Deploy",
    file: "deploy.yml",
    inputs: [
      { key: "version", label: "Version", type: "text", def: "", ph: "latest" },
      { key: "dry_run", label: "Dry run", hint: "Plan only, skip apply", type: "bool", def: false },
    ],
  },
  {
    name: "Release",
    file: "release.yml",
    inputs: [
      {
        key: "bump",
        label: "Version bump",
        type: "choice",
        options: ["patch", "minor", "major"],
        def: "patch",
      },
      {
        key: "prerelease",
        label: "Pre-release",
        hint: "Mark as not production-ready",
        type: "bool",
        def: false,
      },
    ],
  },
  { name: "Nightly build", file: "nightly.yml", inputs: [] },
];

export const BRANCHES = ["main", "develop", "feat/invites", "fix/auth-refresh"];

export const PULL_REQUESTS: PullRequest[] = [
  { n: 214, title: "Workspace invites via email", author: "ayu", head: "feat/invites", draft: false, review: true, checks: "success", updated: "2h" },
  { n: 213, title: "Fix token refresh race condition", author: "dimas", head: "fix/auth-refresh", draft: false, review: true, checks: "failure", updated: "12m" },
  { n: 211, title: "Migrate settings page to new form kit", author: "rizky", head: "refactor/settings", draft: true, review: false, checks: "pending", updated: "1d" },
  { n: 209, title: "Bump dependencies (October)", author: "renovate", head: "renovate/all", draft: false, review: false, checks: "success", updated: "6d" },
  { n: 207, title: "Dark mode for email templates", author: "sari", head: "feat/email-dark", draft: true, review: false, checks: "success", updated: "3d" },
  { n: 205, title: "Add rate limit headers", author: "dimas", head: "feat/rate-limit", draft: false, review: false, checks: "success", updated: "4d" },
];

/** Dot colour token + label for a PR check state. */
export const CHECK_STATES: Record<CheckStatus, { color: string; label: string }> = {
  success: { color: "bg-green-dot", label: "Checks passed" },
  failure: { color: "bg-red-dot", label: "Checks failed" },
  pending: { color: "bg-orange-dot", label: "Checks running" },
};

/** Default input values for a workflow, keyed by input key. */
export function defaultValues(workflow: Workflow): Record<string, InputValue> {
  return Object.fromEntries(workflow.inputs.map((i) => [i.key, i.def]));
}
