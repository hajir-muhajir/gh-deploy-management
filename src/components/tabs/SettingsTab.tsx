import clsx from "clsx";
import type { ReactNode } from "react";
import { EyeIcon } from "../icons";
import { SegmentedControl } from "../ui/SegmentedControl";
import { Toggle } from "../ui/Toggle";
import type { AccentId, Repo, ThemePreference } from "../../types";

export interface WorkflowRow {
  name: string;
  file: string;
  count: number;
  visible: boolean;
}

interface SettingsTabProps {
  token: string;
  draft: string;
  showToken: boolean;
  onDraftChange: (value: string) => void;
  onToggleShowToken: () => void;
  onSaveToken: () => void;

  repos: Repo[];
  onToggleRepo: (index: number) => void;
  newRepo: string;
  onNewRepoChange: (value: string) => void;
  onAddRepo: () => void;

  workflowRows: WorkflowRow[];
  onToggleWorkflow: (name: string) => void;

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

export function SettingsTab(props: SettingsTabProps) {
  const connected = props.token.length >= 20;
  const dirty = props.draft.trim() !== props.token;
  const visibleRepos = props.repos.filter((r) => r.visible).length;
  const visibleWorkflows = props.workflowRows.filter((w) => w.visible).length;

  const tokenStatus = connected
    ? "Connected as indotaichen"
    : props.token
      ? "Token looks invalid"
      : "Not connected";

  return (
    <div className="flex flex-col gap-[18px] px-2.5 pb-3 pt-1">
      <Section title="GitHub access">
        <div className={clsx(CARD, "flex flex-col gap-2.5 p-3")}>
          <div className="flex items-center gap-2">
            <span
              className={clsx(
                "h-[7px] w-[7px] flex-none rounded-full",
                connected ? "bg-green-dot" : "bg-red-dot",
              )}
            />
            <span className="min-w-0 flex-1 truncate text-[13px] font-medium">{tokenStatus}</span>
          </div>

          <div className="flex gap-1.5">
            <div className="flex min-w-0 flex-1 items-center rounded-[7px] bg-fill">
              <input
                type={props.showToken ? "text" : "password"}
                value={props.draft}
                onChange={(e) => props.onDraftChange(e.target.value)}
                placeholder="ghp_... or github_pat_..."
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
              className={clsx(
                "cursor-pointer rounded-[7px] border-0 bg-accent px-3 text-xs font-semibold text-white",
                dirty ? "opacity-100" : "opacity-40",
              )}
            >
              Save
            </button>
          </div>

          <div className="flex justify-between gap-2 text-[11px] text-fg2">
            <span className="min-w-0 truncate">Scopes: repo, workflow</span>
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
          {props.repos.map((repo, index) => (
            <div key={`${repo.owner}/${repo.name}`} className={ROW}>
              <span className="grid h-6 w-6 flex-none place-items-center rounded-md bg-fill font-mono text-[9px] font-semibold">
                {repo.ini}
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-px">
                <span className="truncate text-[13px]">{repo.name}</span>
                <span className="text-[11px] text-fg2">{repo.owner}</span>
              </span>
              <Toggle
                label={`Show ${repo.name}`}
                checked={repo.visible}
                onChange={() => props.onToggleRepo(index)}
              />
            </div>
          ))}
          <div className="flex gap-1.5 border-t-[0.5px] border-sep px-3 py-2">
            <input
              value={props.newRepo}
              onChange={(e) => props.onNewRepoChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") props.onAddRepo();
              }}
              placeholder="owner/repository"
              spellCheck={false}
              className="min-w-0 flex-1 rounded-md border-0 bg-fill px-2 py-1.5 font-mono text-xs text-fg outline-none"
            />
            <button
              type="button"
              onClick={props.onAddRepo}
              className="cursor-pointer rounded-md border-0 bg-fill px-3 text-xs font-semibold text-accent"
            >
              Add
            </button>
          </div>
        </div>
        <span className="pl-1 text-[11px] text-fg2">
          {`${visibleRepos} of ${props.repos.length} shown in the repository switcher`}
        </span>
      </Section>

      <Section title="Workflows in Actions">
        <div className={clsx(CARD, "overflow-hidden")}>
          {props.workflowRows.map((row) => (
            <div key={row.name} className={ROW}>
              <span className="flex min-w-0 flex-1 flex-col gap-px">
                <span className="text-[13px]">{row.name}</span>
                <span className="truncate font-mono text-[11px] text-fg2">{row.file}</span>
              </span>
              <span className="whitespace-nowrap text-[11px] text-fg3">{`${row.count} runs`}</span>
              <Toggle
                label={`Show ${row.name}`}
                checked={row.visible}
                onChange={() => props.onToggleWorkflow(row.name)}
              />
            </div>
          ))}
        </div>
        <span className="pl-1 text-[11px] text-fg2">
          {`${visibleWorkflows} of ${props.workflowRows.length} workflows shown in Actions`}
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
