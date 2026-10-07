import clsx from "clsx";
import {
  ChevronDownIcon,
  ExternalLinkIcon,
  RefreshIcon,
  SettingsIcon,
} from "../icons";
import { IconButton } from "../ui/IconButton";
import type { Repo } from "../../types";

interface FlyoutHeaderProps {
  repo: Repo;
  repoUrl: string;
  /** Bumped on every refresh click to restart the icon spin animation. */
  spinKey: number;
  onRefresh: () => void;
  onToggleMenu: () => void;
  onOpenSettings: () => void;
  settingsActive: boolean;
}

export function FlyoutHeader({
  repo,
  repoUrl,
  spinKey,
  onRefresh,
  onToggleMenu,
  onOpenSettings,
  settingsActive,
}: FlyoutHeaderProps) {
  return (
    <div className="flex items-center gap-1.5 pb-2 pl-3 pr-2.5 pt-3">
      <button
        type="button"
        onClick={onToggleMenu}
        className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 rounded-lg border-0 bg-transparent p-1 text-left text-fg hover:bg-hover"
      >
        <div className="grid h-[30px] w-[30px] flex-none place-items-center rounded-lg bg-fg font-mono text-[11px] font-semibold leading-none text-solid">
          {repo.ini}
        </div>
        <div className="flex min-w-0 flex-col gap-px">
          <span className="truncate text-[11px] text-fg2">{repo.owner}</span>
          <span className="flex items-center gap-[5px] text-sm font-semibold tracking-[-0.01em]">
            <span className="truncate">{repo.name}</span>
            <ChevronDownIcon className="flex-none text-fg3" />
          </span>
        </div>
      </button>

      <IconButton title="Refresh" onClick={onRefresh}>
        <RefreshIcon key={spinKey} className={clsx(spinKey > 0 && "animate-ghturn")} />
      </IconButton>
      <IconButton title="Settings" onClick={onOpenSettings} active={settingsActive}>
        <SettingsIcon />
      </IconButton>
      <IconButton title="Open on GitHub" href={repoUrl}>
        <ExternalLinkIcon />
      </IconButton>
    </div>
  );
}
