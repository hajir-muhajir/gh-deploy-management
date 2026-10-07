import clsx from "clsx";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
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
import * as api from "../../lib/github";
import type { DispatchWorkflow } from "../../lib/github";
import { intervalMs, useAutoRefresh } from "../../hooks/useAutoRefresh";
import { useDispatchable } from "../../hooks/useDispatchable";
import { useFlyoutShown } from "../../hooks/useFlyoutShown";
import { useGitHubAuth } from "../../hooks/useGitHubAuth";
import { usePulls } from "../../hooks/usePulls";
import { useReleases } from "../../hooks/useReleases";
import { useIsTauri } from "../../hooks/useIsTauri";
import { useRepos } from "../../hooks/useRepos";
import { useRuns } from "../../hooks/useRuns";
import type { useTheme } from "../../hooks/useTheme";
import { useToast } from "../../hooks/useToast";
import { useWorkflows } from "../../hooks/useWorkflows";
import type { PrFilterId, Repo, ViewId } from "../../types";

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
  const isTauri = useIsTauri();
  const auth = useGitHubAuth();
  const connected = auth.status === "connected";
  const repos = useRepos(connected);

  const [activeFullName, setActiveFullName] = useState<string | null>(null);
  const [view, setView] = useState<ViewId>("actions");
  const [menuOpen, setMenuOpen] = useState(false);
  const [prFilter, setPrFilter] = useState<PrFilterId>("open");
  const [spin, setSpin] = useState(0);

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
  const pulls = usePulls(activeFullName, connected);
  // Lazy: one request per active workflow, so only while the tab is open.
  const dispatchable = useDispatchable(activeFullName, connected && view === "run");

  const runs = useMemo(
    () => runsState.runs.filter((run) => !workflows.hiddenWorkflowIds.has(run.workflowId)),
    [runsState.runs, workflows.hiddenWorkflowIds],
  );

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
  // Only what is actually waiting on this account, so the tray badge means
  // something: a PR with any reviewer requested would be mostly noise.
  const viewerLogin = auth.info?.login ?? "";
  const reviewCount = viewerLogin
    ? pulls.pulls.filter((pr) => pr.requestedReviewers.includes(viewerLogin)).length
    : 0;

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

  const badgeCount = failingCount + reviewCount;
  useEffect(() => onBadgeChange?.(badgeCount), [badgeCount, onBadgeChange]);

  // What the footer means by "synced": the two sets the timer keeps current.
  // Including releases here would report the age of data that never ticks.
  // ISO strings sort lexicographically, so the last one is the newest.
  const synced = [runsState.syncedAt, pulls.syncedAt]
    .filter((at): at is string => at !== null)
    .sort();
  const syncedAt = synced.length > 0 ? synced[synced.length - 1] : null;

  const autoRefresh = useAutoRefresh();
  const period = intervalMs(autoRefresh.interval);

  const refreshLive = useCallback(() => {
    runsState.refresh();
    pulls.refresh();
  }, [runsState.refresh, pulls.refresh]);

  const refreshAll = useCallback(() => {
    setSpin((s) => s + 1);
    repos.reload();
    workflows.reload();
    releases.refresh();
    // A no-op unless the Run tab is open; the hook is gated on that.
    dispatchable.reload();
    refreshLive();
  }, [repos.reload, workflows.reload, releases.refresh, dispatchable.reload, refreshLive]);

  // Only runs and pull requests tick: they are what the badge is made of, and
  // the rest barely changes between manual refreshes.
  useEffect(() => {
    if (!connected || period === null) return;
    const id = setInterval(refreshLive, period);
    return () => clearInterval(id);
  }, [connected, period, refreshLive]);

  // Opening the flyout is the one moment worth paying for everything.
  useFlyoutShown(isTauri && connected, refreshAll);

  /**
   * Starts a workflow for real. GitHub needs a few seconds before the new run
   * shows up in the runs list, so the Actions tab is refreshed rather than
   * pretending the run is already there — the header refresh is the way out.
   */
  async function dispatch(workflow: DispatchWorkflow, inputs: Record<string, string>) {
    if (!repo) return;
    try {
      await api.dispatchWorkflow(repo.owner, repo.name, workflow.id, repo.defaultBranch, inputs);
      showToast(`${workflow.name} started`);
      setView("actions");
      runsState.refresh();
    } catch (caught) {
      showToast(api.toApiError(caught).message);
    }
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
      refreshInterval={autoRefresh.interval}
      onRefreshIntervalChange={autoRefresh.setInterval}
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
            workflows={dispatchable.workflows}
            loading={dispatchable.loading}
            error={dispatchable.error}
            defaultBranch={repo.defaultBranch}
            onDispatch={dispatch}
          />
        );
      case "prs":
        return (
          <PullRequestsTab
            pulls={pulls.pulls}
            hasMore={pulls.hasMore}
            loading={pulls.loading}
            error={pulls.error}
            viewerLogin={viewerLogin}
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
          onRefresh={refreshAll}
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

      <FlyoutFooter repoUrl={repoUrl} syncedAt={syncedAt} />
      <Toast message={message} />
    </div>
  );
}
