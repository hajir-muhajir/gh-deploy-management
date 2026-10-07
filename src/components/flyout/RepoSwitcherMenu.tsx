import clsx from "clsx";
import { CheckIcon } from "../icons";
import type { Repo } from "../../types";

interface RepoSwitcherMenuProps {
  repos: Repo[];
  activeFullName: string | null;
  /** Failing run count per repository, keyed by full name. */
  failingCounts: Record<string, number>;
  onPick: (fullName: string) => void;
  onManage: () => void;
}

export function RepoSwitcherMenu({
  repos,
  activeFullName,
  failingCounts,
  onPick,
  onManage,
}: RepoSwitcherMenuProps) {
  // Hidden repos stay listed while they are the active one, matching the design.
  const entries = repos.filter((repo) => repo.visible || repo.fullName === activeFullName);

  return (
    <div className="absolute left-3 top-[52px] z-20 flex w-60 flex-col rounded-[10px] bg-solid p-[5px] shadow-flyout">
      {entries.map((repo) => {
        const failing = failingCounts[repo.fullName] ?? 0;
        const active = repo.fullName === activeFullName;
        return (
          <button
            key={repo.fullName}
            type="button"
            onClick={() => onPick(repo.fullName)}
            className="flex cursor-pointer items-center gap-[9px] rounded-md border-0 bg-transparent px-2 py-[7px] text-left text-fg hover:bg-hover"
          >
            <span className="grid h-[22px] w-[22px] flex-none place-items-center rounded-md bg-fill font-mono text-[9px] font-semibold">
              {repo.ini}
            </span>
            <span className="min-w-0 flex-1 truncate text-[13px]">{repo.name}</span>
            <span className="flex-none text-[11px] text-fg3">
              {failing ? `${failing} failing` : ""}
            </span>
            <CheckIcon
              className={clsx("flex-none text-accent", active ? "opacity-100" : "opacity-0")}
            />
          </button>
        );
      })}
      <div className="mx-1.5 my-1 h-px bg-sep" />
      <button
        type="button"
        onClick={onManage}
        className="flex cursor-pointer items-center rounded-md border-0 bg-transparent px-2 py-[7px] text-left text-[13px] text-accent hover:bg-hover"
      >
        Manage repositories…
      </button>
    </div>
  );
}
