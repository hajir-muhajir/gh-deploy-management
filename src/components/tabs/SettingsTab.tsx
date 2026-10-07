import clsx from "clsx";
import type { ReactNode } from "react";
import { EyeIcon } from "../icons";
import { RepoPicker } from "../ui/RepoPicker";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Toggle } from "../ui/Toggle";
import type { GitHubAuth } from "../../hooks/useGitHubAuth";
import type { UseRepos } from "../../hooks/useRepos";
import type { UseWorkflows } from "../../hooks/useWorkflows";
import type { AccentId, RefreshInterval, ThemePreference } from "../../types";

interface SettingsTabProps {
  auth: GitHubAuth;
  draft: string;
  showToken: boolean;
  onDraftChange: (value: string) => void;
  onToggleShowToken: () => void;
  onSaveToken: () => void;

  repos: UseRepos;
  workflows: UseWorkflows;
  /** Null until a repository is selected; drives the empty copy below. */
  activeRepoName: string | null;

  refreshInterval: RefreshInterval;
  onRefreshIntervalChange: (value: RefreshInterval) => void;

  autostart: boolean;
  onAutostartChange: (value: boolean) => void;

  themePreference: ThemePreference;
  onThemeChange: (value: ThemePreference) => void;
  accent: AccentId;
  onAccentChange: (value: AccentId) => void;
}

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-1.5">
      <span className="pl-1 text-[11px] font-medium text-fg2">{title}</span>
      {children}
    </section>
  );
}

const CARD = "rounded-[10px] bg-card shadow-[inset_0_0_0_0.5px_var(--sep)]";
const ROW = "flex items-center gap-2.5 border-t-[0.5px] border-sep px-3 py-2 first:border-t-0";

type DotTone = "green" | "red" | "orange" | "idle";

const DOT_CLASS: Record<DotTone, string> = {
  green: "bg-green-dot",
  red: "bg-red-dot",
  orange: "bg-orange-dot",
  idle: "bg-fg3",
};

/** Maps the auth state onto the status row of the "GitHub access" card. */
function tokenStatus(auth: GitHubAuth): { tone: DotTone; text: string } {
  switch (auth.status) {
    case "loading":
      return { tone: "idle", text: "Checking token…" };
    case "disconnected":
      return { tone: "red", text: "Not connected" };
    case "invalid":
      return { tone: "red", text: `Invalid token — ${auth.error?.message ?? "rejected by GitHub"}` };
    case "error": {
      // An "unknown" kind carries a raw thrown value, which is not fit for the UI.
      const known = auth.error && auth.error.kind !== "unknown";
      return { tone: "orange", text: known ? auth.error!.message : "Cannot reach GitHub" };
    }
    case "connected": {
      const info = auth.info;
      if (!info) return { tone: "orange", text: "Connected" };
      if (info.missingScopes.length > 0) {
        return {
          tone: "orange",
          text: `Connected as ${info.login} · missing scope: ${info.missingScopes.join(", ")}`,
        };
      }
      if (!info.reposVerified) {
        return { tone: "orange", text: `Connected as ${info.login} · no repository access` };
      }
      return { tone: "green", text: `Connected as ${info.login}` };
    }
  }
}

/**
 * Fine-grained tokens do not expose their permissions, so the line reports what
 * the probes actually confirmed instead of a fixed scope list.
 */
function permissionLine(auth: GitHubAuth): string {
  const info = auth.info;
  if (!info) return "Needs repo and workflow access";
  if (info.kind === "classic") {
    return info.scopes.length > 0 ? `Scopes: ${info.scopes.join(", ")}` : "Scopes: none";
  }
  const mark = (ok: boolean) => (ok ? "✓" : "✕");
  const actions = info.actionsVerified ? "✓" : "—";
  return `Fine-grained · repositories ${mark(info.reposVerified)} · actions ${actions}`;
}

export function SettingsTab(props: SettingsTabProps) {
  const { auth, repos, workflows } = props;
  const status = tokenStatus(auth);
  const dirty = props.draft.trim() !== "";
  const canSave = !auth.saving && (dirty || auth.status === "connected");

  const trackedKeys = new Set(repos.tracked.map((r) => r.fullName.toLowerCase()));
  const visibleRepos = repos.tracked.filter((r) => r.visible).length;
  const visibleWorkflows = workflows.workflows.filter((w) => w.visible).length;

  return (
    <div className="flex flex-col gap-[18px] px-2.5 pb-3 pt-1">
      <Section title="GitHub access">
        <div className={clsx(CARD, "flex flex-col gap-2.5 p-3")}>
          <div className="flex items-center gap-2">
            <span className={clsx("h-[7px] w-[7px] flex-none rounded-full", DOT_CLASS[status.tone])} />
            <span className="min-w-0 flex-1 truncate text-[13px] font-medium" title={status.text}>
              {status.text}
            </span>
          </div>

          <div className="flex gap-1.5">
            <div className="flex min-w-0 flex-1 items-center rounded-[7px] bg-fill">
              <input
                type={props.showToken ? "text" : "password"}
                value={props.draft}
                onChange={(e) => props.onDraftChange(e.target.value)}
                placeholder={
                  auth.status === "connected" ? "Replace token…" : "ghp_... or github_pat_..."
                }
                spellCheck={false}
                className="min-w-0 flex-1 border-0 bg-transparent px-2 py-[7px] font-mono text-xs text-fg outline-none"
              />
              <button
                type="button"
                title={props.showToken ? "Hide token" : "Show token"}
                onClick={props.onToggleShowToken}
                className="grid h-7 w-7 flex-none cursor-pointer place-items-center border-0 bg-transparent p-0 text-fg2 hover:text-fg"
              >
                <EyeIcon />
              </button>
            </div>
            <button
              type="button"
              onClick={props.onSaveToken}
              disabled={!canSave}
              className={clsx(
                "cursor-pointer whitespace-nowrap rounded-[7px] border-0 bg-accent px-3 text-xs font-semibold text-white",
                canSave ? "opacity-100" : "cursor-default opacity-40",
              )}
            >
              {auth.saving ? "Checking…" : !dirty && auth.status === "connected" ? "Remove" : "Save"}
            </button>
          </div>

          <div className="flex justify-between gap-2 text-[11px] text-fg2">
            <span className="min-w-0 truncate" title={permissionLine(auth)}>
              {permissionLine(auth)}
            </span>
            <a
              href="https://github.com/settings/tokens"
              target="_blank"
              rel="noreferrer"
              className="flex-none whitespace-nowrap text-accent hover:underline"
            >
              Create token
            </a>
          </div>
        </div>
      </Section>

      <Section title="Repositories in list">
        <div className={clsx(CARD, "overflow-hidden")}>
          {repos.tracked.map((repo) => (
            <div key={repo.fullName} className={ROW}>
              <span className="grid h-6 w-6 flex-none place-items-center rounded-md bg-fill font-mono text-[9px] font-semibold">
                {repo.ini}
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-px">
                <span className="truncate text-[13px]">{repo.name}</span>
                <span className="truncate text-[11px] text-fg2">{repo.owner}</span>
              </span>
              <button
                type="button"
                title={`Remove ${repo.name}`}
                onClick={() => repos.remove(repo.fullName)}
                className="flex-none cursor-pointer border-0 bg-transparent px-1 text-[11px] text-fg3 hover:text-fg"
              >
                Remove
              </button>
              <Toggle
                label={`Show ${repo.name}`}
                checked={repo.visible}
                onChange={() => repos.toggleVisible(repo.fullName)}
              />
            </div>
          ))}

          {repos.tracked.length === 0 && (
            <p className="px-3 py-3 text-[11px] text-fg2">
              {auth.status === "connected"
                ? "No repositories yet — search below to add one."
                : "Save a token first to browse your repositories."}
            </p>
          )}

          <RepoPicker
            available={repos.available}
            trackedKeys={trackedKeys}
            onAdd={repos.add}
            disabled={auth.status !== "connected"}
          />
        </div>

        <span className="pl-1 text-[11px] text-fg2">
          {repos.loading
            ? "Loading repositories…"
            : repos.error
              ? repos.error.message
              : `${visibleRepos} of ${repos.tracked.length} shown in the repository switcher`}
          {repos.truncated && " · only the most recently pushed repositories are searchable"}
        </span>
      </Section>

      <Section title="Workflows in Actions">
        <div className={clsx(CARD, "overflow-hidden")}>
          {/* Keyed by path: a repo can have two workflows with the same name. */}
          {workflows.workflows.map((workflow) => (
            <div key={workflow.path} className={ROW}>
              <span className="flex min-w-0 flex-1 flex-col gap-px">
                <span className="truncate text-[13px]" title={workflow.name}>
                  {workflow.name}
                </span>
                <span className="truncate font-mono text-[11px] text-fg2" title={workflow.path}>
                  {workflow.path}
                </span>
              </span>
              {!workflow.active && (
                <span className="whitespace-nowrap text-[11px] text-fg3">disabled</span>
              )}
              <Toggle
                label={`Show ${workflow.name}`}
                checked={workflow.visible}
                onChange={() => workflows.toggle(workflow.path)}
              />
            </div>
          ))}

          {workflows.workflows.length === 0 && (
            <p className="px-3 py-3 text-[11px] text-fg2">
              {!props.activeRepoName
                ? "Pick a repository first."
                : workflows.loading
                  ? "Loading workflows…"
                  : workflows.error
                    ? workflows.error.message
                    : "This repository has no workflows."}
            </p>
          )}
        </div>
        <span className="pl-1 text-[11px] text-fg2">
          {workflows.workflows.length > 0
            ? `${visibleWorkflows} of ${workflows.workflows.length} workflows shown in Actions`
            : props.activeRepoName
              ? `From ${props.activeRepoName}`
              : ""}
        </span>
      </Section>

      <Section title="Refresh">
        <div className={clsx(CARD, "p-3")}>
          <SegmentedControl<RefreshInterval>
            options={[
              { value: "off", label: "Off" },
              { value: "1m", label: "1m" },
              { value: "5m", label: "5m" },
              { value: "15m", label: "15m" },
            ]}
            value={props.refreshInterval}
            onChange={props.onRefreshIntervalChange}
          />
        </div>
        <span className="pl-1 text-[11px] text-fg2">
          {props.refreshInterval === "off"
            ? "Runs and pull requests only reload when you press refresh."
            : "Runs and pull requests reload on their own. Releases, workflows and the repository list only reload when you press refresh or reopen the panel."}
        </span>
      </Section>

      <Section title="Startup">
        <div className={CARD}>
          <div className={ROW}>
            <span className="min-w-0 flex-1 text-[13px]">Start with Windows</span>
            <Toggle
              label="Start with Windows"
              checked={props.autostart}
              onChange={() => props.onAutostartChange(!props.autostart)}
            />
          </div>
        </div>
        <span className="pl-1 text-[11px] text-fg2">
          Runs in the system tray when you sign in. The panel stays hidden until you click the icon.
        </span>
      </Section>

      <Section title="Appearance">
        <div className={clsx(CARD, "flex flex-col gap-3 p-3")}>
          <div className="flex flex-col gap-1.5">
            <span className="text-[13px]">Theme</span>
            <SegmentedControl<ThemePreference>
              options={[
                { value: "system", label: "System" },
                { value: "light", label: "Light" },
                { value: "dark", label: "Dark" },
              ]}
              value={props.themePreference}
              onChange={props.onThemeChange}
            />
          </div>
          <div className="flex flex-col gap-1.5">
            <span className="text-[13px]">Accent</span>
            <SegmentedControl<AccentId>
              options={[
                { value: "blue", label: "Blue" },
                { value: "indigo", label: "Indigo" },
                { value: "graphite", label: "Graphite" },
              ]}
              value={props.accent}
              onChange={props.onAccentChange}
            />
          </div>
        </div>
      </Section>
    </div>
  );
}
