import type {
  CheckStatus,
  InputValue,
  PullRequest,
  Release,
  Repo,
  Workflow,
  WorkflowRun,
} from "../types";

export const DUMMY_TOKEN = "ghp_8fK2xQ9mVt4LpZ7cR1sN6wY3hJ0dE5aBuG";

export const REPOS: Repo[] = [
  { owner: "indotaichen", name: "web-app", ini: "WA", ver: [2, 4, 1], visible: true },
  { owner: "indotaichen", name: "api-gateway", ini: "AG", ver: [1, 9, 3], visible: true },
  { owner: "indotaichen", name: "mobile-app", ini: "MA", ver: [3, 1, 2], visible: true },
  { owner: "indotaichen", name: "infra", ini: "IN", ver: [0, 12, 4], visible: false },
];

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

export function makeReleases(repo: Repo): Release[] {
  const [M, m, p] = repo.ver;
  return [
    { tag: `v${M}.${m}.${p}`, name: "Faster cold starts", when: "4d ago", author: "rizky", latest: true, notes: ["Lazy-load route bundles", "Warm caches on deploy", "Fix memory leak in session store"] },
    { tag: `v${M}.${m}.0`, name: "Workspace sharing", when: "Sep 26", author: "ayu", notes: ["Invite members by email", "Shared workspace settings"] },
    { tag: `v${M}.${m}.0-rc.2`, name: "Workspace sharing RC 2", when: "Sep 22", author: "ayu", pre: true, notes: ["Release candidate for workspace sharing"] },
    { tag: `v${M}.${m - 1}.7`, name: "Auth refresh patch", when: "Sep 12", author: "dimas", notes: ["Fix token refresh on slow networks"] },
    { tag: `v${M}.${m - 1}.6`, name: "Dependency updates", when: "Aug 30", author: "renovate", notes: ["Bump runtime dependencies"] },
  ];
}

export function makeRuns(index: number, repo: Repo): WorkflowRun[] {
  const b = 480 + index * 37;
  return [
    { wf: "Deploy", title: "Deploy to staging", branch: "main", n: b + 2, status: "running", pct: 38, ago: "started 1m ago", dur: "" },
    { wf: "CI", title: "fix: token refresh race", branch: "fix/auth-refresh", n: b + 1, status: "failure", ago: "12m ago", dur: "3m 41s" },
    { wf: "CI", title: "feat: workspace invites", branch: "feat/invites", n: b, status: "success", ago: "34m ago", dur: "4m 02s" },
    { wf: "Release", title: `Release ${makeReleases(repo)[0].tag}`, branch: "main", n: b - 1, status: "success", ago: "4d ago", dur: "6m 18s" },
    { wf: "Nightly build", title: "Nightly build", branch: "main", n: b - 2, status: "cancelled", ago: "5d ago", dur: "1m 10s" },
    { wf: "CI", title: "chore: bump dependencies", branch: "renovate/all", n: b - 3, status: "success", ago: "6d ago", dur: "3m 55s" },
  ];
}

/** Default input values for a workflow, keyed by input key. */
export function defaultValues(workflow: Workflow): Record<string, InputValue> {
  return Object.fromEntries(workflow.inputs.map((i) => [i.key, i.def]));
}

/** `.github/workflows/<file>` for a workflow name, including runs not in WORKFLOWS (e.g. CI). */
export function workflowFile(name: string): string {
  if (name === "CI") return ".github/workflows/ci.yml";
  const wf = WORKFLOWS.find((w) => w.name === name);
  return `.github/workflows/${wf ? wf.file : "unknown.yml"}`;
}
