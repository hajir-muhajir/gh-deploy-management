import clsx from "clsx";
import { useEffect, useMemo, useState } from "react";
import { FlyoutFooter } from "./FlyoutFooter";
import { FlyoutHeader } from "./FlyoutHeader";
import { RepoSwitcherMenu } from "./RepoSwitcherMenu";
import { TabBar } from "./TabBar";
import { Toast } from "./Toast";
import { ActionsTab } from "../tabs/ActionsTab";
import { PullRequestsTab } from "../tabs/PullRequestsTab";
import { ReleasesTab } from "../tabs/ReleasesTab";
import { RunTab } from "../tabs/RunTab";
import { SettingsTab, type WorkflowRow } from "../tabs/SettingsTab";
import {
  DUMMY_TOKEN,
  PULL_REQUESTS,
  REPOS,
  WORKFLOWS,
  defaultValues,
  makeReleases,
  makeRuns,
  workflowFile,
} from "../../data/dummy";
import type { useTheme } from "../../hooks/useTheme";
import { useToast } from "../../hooks/useToast";
import type { InputValue, PrFilterId, Repo, ViewId, WorkflowRun } from "../../types";

interface FlyoutPanelProps {
  theme: ReturnType<typeof useTheme>;
  /** False collapses the panel with the open/close transition (preview backdrop only). */
  open: boolean;
  /** Anchors the panel bottom-right like the design; off inside the Tauri window. */
  anchored: boolean;
  /** Called with the failing + review-requested total so the backdrop can badge the tray. */
  onBadgeChange?: (count: number) => void;
}

const REPO_PATTERN = /^([\w.-]+)\/([\w.-]+)$/;

export function FlyoutPanel({ theme, open, anchored, onBadgeChange }: FlyoutPanelProps) {
  const [repos, setRepos] = useState<Repo[]>(() => REPOS.map((r) => ({ ...r })));
  const [repoIndex, setRepoIndex] = useState(0);
  const [runsByRepo, setRunsByRepo] = useState<WorkflowRun[][]>(() =>
    REPOS.map((repo, i) => makeRuns(i, repo)),
  );

  const [view, setView] = useState<ViewId>("actions");
  const [menuOpen, setMenuOpen] = useState(false);
  const [prFilter, setPrFilter] = useState<PrFilterId>("open");
  const [spin, setSpin] = useState(0);

  const [workflowIndex, setWorkflowIndex] = useState(0);
  const [branch, setBranch] = useState("main");
  const [values, setValues] = useState<Record<string, InputValue>>(() =>
    defaultValues(WORKFLOWS[0]),
  );

  const [token, setToken] = useState(DUMMY_TOKEN);
  const [draft, setDraft] = useState(DUMMY_TOKEN);
  const [showToken, setShowToken] = useState(false);
  const [newRepo, setNewRepo] = useState("");
  const [hiddenWorkflows, setHiddenWorkflows] = useState<Record<string, boolean>>({
    "Nightly build": true,
  });

  const { message, showToast, copy } = useToast();

  const repo = repos[repoIndex];
  const repoUrl = `https://github.com/${repo.owner}/${repo.name}`;
  const allRuns = runsByRepo[repoIndex] ?? [];
  const runs = allRuns.filter((run) => !hiddenWorkflows[run.wf]);

  const failingCount = runs.filter((r) => r.status === "failure").length;
  const reviewCount = PULL_REQUESTS.filter((p) => p.review).length;
  const failingPerRepo = runsByRepo.map(
    (list) => list.filter((r) => r.status === "failure" && !hiddenWorkflows[r.wf]).length,
  );

  const releases = useMemo(() => makeReleases(repo), [repo]);

  const workflowRows: WorkflowRow[] = useMemo(() => {
    const names = [...new Set([...allRuns.map((r) => r.wf), ...WORKFLOWS.map((w) => w.name)])];
    return names.map((name) => ({
      name,
      file: workflowFile(name),
      count: allRuns.filter((r) => r.wf === name).length,
      visible: !hiddenWorkflows[name],
    }));
  }, [allRuns, hiddenWorkflows]);

  const badgeCount = failingCount + reviewCount;
  useEffect(() => onBadgeChange?.(badgeCount), [badgeCount, onBadgeChange]);

  /** Replace one run of the active repo. */
  function patchRun(runNumber: number, patch: Partial<WorkflowRun>) {
    setRunsByRepo((prev) =>
      prev.map((list, i) =>
        i === repoIndex ? list.map((r) => (r.n === runNumber ? { ...r, ...patch } : r)) : list,
      ),
    );
  }

  function selectWorkflow(index: number) {
    setWorkflowIndex(index);
    setValues(defaultValues(WORKFLOWS[index]));
  }

  function submitRun() {
    const workflow = WORKFLOWS[workflowIndex];
    let title = workflow.name;
    if (workflow.name === "Deploy") {
      title = `Deploy${values.version ? ` ${values.version}` : ""}${values.dry_run ? " (dry run)" : ""}`;
    } else if (workflow.name === "Release") {
      title = `Release · ${values.bump} bump${values.prerelease ? " (pre)" : ""}`;
    }

    setRunsByRepo((prev) =>
      prev.map((list, i) => {
        if (i !== repoIndex) return list;
        const nextNumber = (list[0]?.n ?? 0) + 1;
        const queued: WorkflowRun = {
          wf: workflow.name,
          title,
          branch,
          n: nextNumber,
          status: "queued",
          pct: 0,
          ago: "just now",
          dur: "",
        };
        return [queued, ...list];
      }),
    );

    setView("actions");
    showToast(`${workflow.name} dispatched on ${branch}`);
  }

  function toggleRepoVisibility(index: number) {
    setRepos((prev) => {
      const shown = prev.filter((r) => r.visible).length;
      if (prev[index].visible && shown <= 1) return prev;
      return prev.map((r, i) => (i === index ? { ...r, visible: !r.visible } : r));
    });
  }

  function addRepo() {
    const value = newRepo.trim();
    const match = value.match(REPO_PATTERN);
    if (!match) {
      showToast("Use owner/repository");
      return;
    }
    if (repos.some((r) => `${r.owner}/${r.name}`.toLowerCase() === value.toLowerCase())) {
      showToast("Already in list");
      return;
    }

    const created: Repo = {
      owner: match[1],
      name: match[2],
      ini: match[2].replace(/[^a-z0-9]/gi, "").slice(0, 2).toUpperCase(),
      ver: [1, 3, 0],
      visible: true,
    };
    setRunsByRepo((prev) => [...prev, makeRuns(prev.length, created)]);
    setRepos((prev) => [...prev, created]);
    setNewRepo("");
    showToast(`Added ${created.name}`);
  }

  function saveToken() {
    const next = draft.trim();
    setToken(next);
    showToast(next ? "Token saved" : "Token removed");
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
          onRefresh={() => setSpin((s) => s + 1)}
          onToggleMenu={() => setMenuOpen((m) => !m)}
          onOpenSettings={() => {
            setView("settings");
            setMenuOpen(false);
          }}
          settingsActive={view === "settings"}
        />

        {menuOpen && (
          <>
            <button
              type="button"
              aria-label="Close repository menu"
              onClick={() => setMenuOpen(false)}
              className="fixed inset-0 z-10 cursor-default border-0 bg-transparent"
            />
            <RepoSwitcherMenu
              repos={repos}
              activeIndex={repoIndex}
              failingCounts={failingPerRepo}
              onPick={(index) => {
                setRepoIndex(index);
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

      <div className="relative flex-1 overflow-auto px-1 pb-2">
        {view === "releases" && (
          <ReleasesTab releases={releases} repoUrl={repoUrl} onCopy={copy} />
        )}

        {view === "actions" && (
          <ActionsTab
            runs={runs}
            onCancel={(n) => patchRun(n, { status: "cancelled", ago: "just now", dur: "—" })}
            onRerun={(n) => patchRun(n, { status: "queued", pct: 0, ago: "just now", dur: "" })}
          />
        )}

        {view === "run" && (
          <RunTab
            workflowIndex={workflowIndex}
            branch={branch}
            values={values}
            onSelectWorkflow={selectWorkflow}
            onBranchChange={setBranch}
            onValueChange={(key, value) => setValues((prev) => ({ ...prev, [key]: value }))}
            onSubmit={submitRun}
          />
        )}

        {view === "prs" && (
          <PullRequestsTab filter={prFilter} onFilterChange={setPrFilter} repoUrl={repoUrl} />
        )}

        {view === "settings" && (
          <SettingsTab
            token={token}
            draft={draft}
            showToken={showToken}
            onDraftChange={setDraft}
            onToggleShowToken={() => setShowToken((s) => !s)}
            onSaveToken={saveToken}
            repos={repos}
            onToggleRepo={toggleRepoVisibility}
            newRepo={newRepo}
            onNewRepoChange={setNewRepo}
            onAddRepo={addRepo}
            workflowRows={workflowRows}
            onToggleWorkflow={(name) =>
              setHiddenWorkflows((prev) => ({ ...prev, [name]: !prev[name] }))
            }
            themePreference={theme.preference}
            onThemeChange={theme.setPreference}
            accent={theme.accent}
            onAccentChange={theme.setAccent}
          />
        )}
      </div>

      <FlyoutFooter repoUrl={repoUrl} />
      <Toast message={message} />
    </div>
  );
}
