import clsx from "clsx";
import { PullRequestIcon } from "../icons";
import { CHECK_STATES, PULL_REQUESTS } from "../../data/dummy";
import type { PrFilterId, PullRequest } from "../../types";

interface PullRequestsTabProps {
  filter: PrFilterId;
  onFilterChange: (filter: PrFilterId) => void;
  repoUrl: string;
}

const FILTERS: { id: PrFilterId; label: string; match: (pr: PullRequest) => boolean }[] = [
  { id: "open", label: "Open", match: (p) => !p.draft },
  { id: "draft", label: "Draft", match: (p) => p.draft },
  { id: "review", label: "Review", match: (p) => p.review },
];

export function PullRequestsTab({ filter, onFilterChange, repoUrl }: PullRequestsTabProps) {
  const active = FILTERS.find((f) => f.id === filter) ?? FILTERS[0];
  const visible = PULL_REQUESTS.filter(active.match);

  return (
    <div className="px-1">
      <div className="flex gap-1.5 px-1.5 pb-2 pt-0.5">
        {FILTERS.map((f) => {
          const on = f.id === filter;
          return (
            <button
              key={f.id}
              type="button"
              onClick={() => onFilterChange(f.id)}
              className={clsx(
                "flex cursor-pointer items-center gap-[5px] rounded-full border-0 px-2.5 py-[5px] text-xs font-medium",
                on ? "bg-fg text-solid" : "bg-fill text-fg",
              )}
            >
              {f.label}
              <span className="tabular-nums opacity-65">{PULL_REQUESTS.filter(f.match).length}</span>
            </button>
          );
        })}
      </div>

      {visible.length === 0 && <p className="px-2 py-3 text-xs text-fg2">Nothing here.</p>}

      {visible.map((pr) => {
        const check = CHECK_STATES[pr.checks];
        return (
          <a
            key={pr.n}
            href={`${repoUrl}/pull/${pr.n}`}
            target="_blank"
            rel="noreferrer"
            className="flex gap-2.5 rounded-lg px-2 py-[9px] text-fg no-underline hover:bg-hover"
          >
            <PullRequestIcon
              className={clsx("mt-px flex-none", pr.draft ? "text-fg3" : "text-green-dot")}
            />
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <div className="truncate text-[13px] font-medium">{pr.title}</div>
              <div className="truncate text-[11px] text-fg2">
                {`${pr.author} · ${pr.head} · ${pr.updated}`}
              </div>
              <div className="mt-[3px] flex items-center gap-2 text-[11px] text-fg2">
                <span className="flex items-center gap-1">
                  <span className={clsx("h-1.5 w-1.5 rounded-full", check.color)} />
                  {check.label}
                </span>
                {pr.review && <span className="font-medium text-accent">Review requested</span>}
              </div>
            </div>
            <span className="flex-none text-[11px] tabular-nums text-fg3">{`#${pr.n}`}</span>
          </a>
        );
      })}
    </div>
  );
}
