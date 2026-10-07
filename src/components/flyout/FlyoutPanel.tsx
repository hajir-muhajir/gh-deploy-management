import clsx from "clsx";
import { useEffect, useMemo, useRef, useState } from "react";
import { EmptyState } from "./EmptyState";
import { FlyoutFooter } from "./FlyoutFooter";
import { FlyoutHeader } from "./FlyoutHeader";
import { RepoSwitcherMenu } from "./RepoSwitcherMenu";
import { TabBar } from "./TabBar";
import { Toast } from "./Toast";
import { ActionsTab } from "../tabs/ActionsTab";
import { PullRequestsTab } from "../tabs/PullRequestsTab";
import { ReleasesTab } from "../tabs/ReleasesTab";
import { RunTab } from "../tabs/RunTab";
import { SettingsTab } from "../tabs/SettingsTab";
import { BRANCHES, PULL_REQUESTS, WORKFLOWS, defaultValues } from "../../data/dummy";
import { useGitHubAuth } from "../../hooks/useGitHubAuth";
import { useReleases } from "../../hooks/useReleases";
import { useRepos } from "../../hooks/useRepos";
import { useRuns } from "../../hooks/useRuns";
import type { useTheme } from "../../hooks/useTheme";
import { useToast } from "../../hooks/useToast";
import { useWorkflows } from "../../hooks/useWorkflows";
import type { InputValue, PrFilterId, Repo, ViewId } from "../../types";

interface FlyoutPanelProps {
  theme: ReturnType<typeof useTheme>;
  /** False collapses the panel with the open/close transition (preview backdrop only). */
  open: boolean;
  /** Anchors the panel bottom-right like the design; off inside the Tauri window. */
  anchored: boolean;
  /** Called with the failing + review-requested total so the backdrop can badge the tray. */
  onBadgeChange?: (count: number) => void;
}

export function FlyoutPanel({ theme, open, anchored, onBadgeChange }: FlyoutPanelProps) {
  const auth = useGitHubAuth();
  const connected = auth.status === "connected";
  const repos = useRepos(connected);

  const [activeFullName, setActiveFullName] = useState<string | null>(null);
  const [view, setView] = useState<ViewId>("actions");
  const [menuOpen, setMenuOpen] = useState(false);
  const [prFilter, setPrFilter] = useState<PrFilterId>("open");
  const [spin, setSpin] = useState(0);

  const [workflowIndex, setWorkflowIndex] = useState(0);
  const [branch, setBranch] = useState<string | null>(null);
  const [values, setValues] = useState<Record<string, InputValue>>(() =>
    defaultValues(WORKFLOWS[0]),
  );

  const [draft, setDraft] = useState("");
  const [showToken, setShowToken] = useState(false);

  const { message, showToast, copy } = useToast();

  const visibleRepos = useMemo(() => repos.tracked.filter((r) => r.visible), [repos.tracked]);

  // Keep the selection pointing at a repo that still exists and is still shown.
  useEffect(() => {
    const stillValid = visibleRepos.some((r) => r.fullName === activeFullName);
    if (stillValid) return;
    setActiveFullName(visibleRepos[0]?.fullName ?? null);
  }, [visibleRepos, activeFullName]);

  const repo: Repo | null = repos.tracked.find((r) => r.fullName === activeFullName) ?? null;
  const repoUrl = repo ? `https://github.com/${repo.fullName}` : null;

  const workflows = useWorkflows(activeFullName, connected);
  const runsState = useRuns(activeFullName, connected);
  const releases = useReleases(activeFullName, connected);

  const runs = useMemo(
    () => runsState.runs.filter((run) => !workflows.hiddenWorkflowIds.has(run.workflowId)),
    [runsState.runs, workflows.hiddenWorkflowIds],
  );

  // Reset the dispatch branch whenever the active repository changes; default
  // branches are not always "main" (some of this account's repos use "Main").
  useEffect(() => {
    setBranch(repo?.defaultBranch ?? null);
  }, [repo?.fullName, repo?.defaultBranch]);

  // Probe the Actions permission once a repository is known — for fine-grained
  // tokens that is the only way to confirm it. Each repo is probed at most once
  // per session so switching back and forth does not re-spend rate limit.
  const probed = useRef(new Set<string>());
  const { status: authStatus, recheck } = auth;
  useEffect(() => {
    const fullName = repo?.fullName;
    if (authStatus !== "connected" || !fullName || probed.current.has(fullName)) return;
    probed.current.add(fullName);
    void recheck(fullName);
  }, [authStatus, recheck, repo?.fullName]);

  const failingCount = runs.filter((r) => r.status === "failure").length;
  const reviewCount = repo ? PULL_REQUESTS.filter((p) => p.review).length : 0;

  /**
   * Only the active repository's runs are fetched, so the switcher can only
   * badge repos that have been opened at least once. Counts are remembered for
   * the session rather than costing one request per tracked repo on every
   * refresh.
   */
  const [failingPerRepo, setFailingPerRepo] = useState<Record<string, number>>({});
  useEffect(() => {
    if (!repo || runsState.loading) return;
    setFailingPerRepo((prev) =>
      prev[repo.fullName] === failingCount ? prev : { ...prev, [repo.fullName]: failingCount },
    );
  }, [repo, failingCount, runsState.loading]);

  // The default branch is not always "main", so put the repo's own first and
  // drop it from the placeholder list to avoid a duplicate option.
  const branchOptions = useMemo(() => {
    if (!repo) return BRANCHES;
    return [repo.defaultBranch, ...BRANCHES.filter((b) => b !== repo.defaultBranch)];
  }, [repo]);

  const badgeCount = failingCount + reviewCount;
  useEffect(() => onBadgeChange?.(badgeCount), [badgeCount, onBadgeChange]);

  function selectWorkflow(index: number) {
    setWorkflowIndex(index);
    setValues(defaultValues(WORKFLOWS[index]));
  }

  /** Placeholder: the Run tab is not wired to workflow_dispatch yet. */
  function submitRun() {
    if (!repo || !branch) return;
    showToast(`${WORKFLOWS[workflowIndex].name} dispatch is not wired up yet`);
  }

  async function saveToken() {
    const next = draft.trim();
    const ok = await auth.save(next, repo?.fullName);
    if (!ok) {
      // Keep the draft so the user can correct it; the status row explains why.
      showToast("Token rejected");
      return;
    }
    setDraft("");
    showToast(next ? "Token saved" : "Token removed");
  }

  const settings = (
    <SettingsTab
      auth={auth}
      draft={draft}
      showToken={showToken}
      onDraftChange={setDraft}
      onToggleShowToken={() => setShowToken((s) => !s)}
      onSaveToken={() => void saveToken()}
      repos={repos}
      workflows={workflows}
      activeRepoName={repo?.fullName ?? null}
      themePreference={theme.preference}
      onThemeChange={theme.setPreference}
      accent={theme.accent}
      onAccentChange={theme.setAccent}
    />
  );

  /**
   * Without a repository the data tabs have nothing to render, so they are
   * replaced by a prompt. Settings stays reachable — otherwise the first run,
   * which has no token and no repos, would be a dead end.
   */
  function renderContent() {
    if (view === "settings") return settings;

    if (auth.status === "loading") {
      return <p className="px-4 py-6 text-center text-xs text-fg2">Loading…</p>;
    }

    if (!connected) {
      return (
        <EmptyState
          title="Connect your GitHub account"
          body="Save a personal access token in Settings to start tracking repositories."
          actionLabel="Open Settings"
          onAction={() => setView("settings")}
        />
      );
    }

    if (!repo) {
      return (
        <EmptyState
          title="No repositories yet"
          body="Pick the repositories you want to watch and they will show up here."
          actionLabel="Choose repositories"
          onAction={() => setView("settings")}
        />
      );
    }

    switch (view) {
      case "releases":
        return (
          <ReleasesTab
            releases={releases.releases}
            hasMore={releases.hasMore}
            loading={releases.loading}
            error={releases.error}
            repoUrl={repoUrl ?? ""}
            onCopy={copy}
            onNotice={showToast}
          />
        );
      case "actions":
        return (
          <ActionsTab
            runs={runs}
            loading={runsState.loading}
            error={runsState.error}
            onCancel={runsState.cancel}
            onRerun={runsState.rerun}
            onActionError={showToast}
          />
        );
      case "run":
        return (
          <RunTab
            workflowIndex={workflowIndex}
            branch={branch ?? repo.defaultBranch}
            branches={branchOptions}
            values={values}
            onSelectWorkflow={selectWorkflow}
            onBranchChange={setBranch}
            onValueChange={(key, value) => setValues((prev) => ({ ...prev, [key]: value }))}
            onSubmit={submitRun}
          />
        );
      case "prs":
        return (
          <PullRequestsTab
            filter={prFilter}
            onFilterChange={setPrFilter}
            repoUrl={repoUrl ?? ""}
          />
        );
    }
  }

  return (
    <div
      className={clsx(
        "flex flex-col overflow-hidden bg-panel transition-[opacity,transform] duration-300 ease-[cubic-bezier(.2,.8,.2,1)]",
        anchored
          ? "absolute bottom-[60px] right-3 h-[608px] w-[360px] origin-bottom-right rounded-xl shadow-flyout backdrop-blur-[40px] backdrop-saturate-[1.8]"
          : "absolute inset-0",
        open ? "scale-100 opacity-100" : "pointer-events-none translate-y-3.5 scale-[0.98] opacity-0",
      )}
    >
      <div className="relative">
        <FlyoutHeader
          repo={repo}
          repoUrl={repoUrl}
          spinKey={spin}
          onRefresh={() => {
            setSpin((s) => s + 1);
            repos.reload();
            workflows.reload();
            runsState.refresh();
            releases.refresh();
          }}
          onToggleMenu={() => setMenuOpen((m) => !m)}
          onOpenSettings={() => {
            setView("settings");
            setMenuOpen(false);
          }}
          settingsActive={view === "settings"}
        />

        {menuOpen && repo && (
          <>
            <button
              type="button"
              aria-label="Close repository menu"
              onClick={() => setMenuOpen(false)}
              className="fixed inset-0 z-10 cursor-default border-0 bg-transparent"
            />
            <RepoSwitcherMenu
              repos={repos.tracked}
              activeFullName={activeFullName}
              failingCounts={failingPerRepo}
              onPick={(fullName) => {
                setActiveFullName(fullName);
                setMenuOpen(false);
              }}
              onManage={() => {
                setView("settings");
                setMenuOpen(false);
              }}
            />
          </>
        )}
      </div>

      <TabBar
        value={view}
        onChange={setView}
        failingCount={failingCount}
        reviewCount={reviewCount}
      />

      <div className="relative flex-1 overflow-auto px-1 pb-2">{renderContent()}</div>

      <FlyoutFooter repoUrl={repoUrl} />
      <Toast message={message} />
    </div>
  );
}
