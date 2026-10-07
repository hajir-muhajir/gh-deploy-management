import type {
  CheckStatus,
  InputValue,
  PullRequest,
  Release,
  Repo,
  Workflow,
} from "../types";

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

/**
 * Stable per-repo seed so the placeholder Releases content differs between
 * repositories and stays the same across renders. Replaced once that tab is
 * wired to the real API.
 */
function seedOf(fullName: string): number {
  let hash = 0;
  for (const char of fullName) hash = (hash * 31 + char.charCodeAt(0)) >>> 0;
  return hash % 1000;
}

function versionOf(repo: Repo): [number, number, number] {
  const seed = seedOf(repo.fullName);
  return [1 + (seed % 3), 1 + (seed % 9), seed % 10];
}

export function makeReleases(repo: Repo): Release[] {
  const [M, m, p] = versionOf(repo);
  return [
    { tag: `v${M}.${m}.${p}`, name: "Faster cold starts", when: "4d ago", author: "rizky", latest: true, notes: ["Lazy-load route bundles", "Warm caches on deploy", "Fix memory leak in session store"] },
    { tag: `v${M}.${m}.0`, name: "Workspace sharing", when: "Sep 26", author: "ayu", notes: ["Invite members by email", "Shared workspace settings"] },
    { tag: `v${M}.${m}.0-rc.2`, name: "Workspace sharing RC 2", when: "Sep 22", author: "ayu", pre: true, notes: ["Release candidate for workspace sharing"] },
    { tag: `v${M}.${m - 1}.7`, name: "Auth refresh patch", when: "Sep 12", author: "dimas", notes: ["Fix token refresh on slow networks"] },
    { tag: `v${M}.${m - 1}.6`, name: "Dependency updates", when: "Aug 30", author: "renovate", notes: ["Bump runtime dependencies"] },
  ];
}

/** Default input values for a workflow, keyed by input key. */
export function defaultValues(workflow: Workflow): Record<string, InputValue> {
  return Object.fromEntries(workflow.inputs.map((i) => [i.key, i.def]));
}
