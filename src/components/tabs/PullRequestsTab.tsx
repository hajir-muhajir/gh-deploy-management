import clsx from "clsx";
import { ExternalLinkIcon, PullRequestIcon } from "../icons";
import type { ApiError, CheckState, PullRequestInfo } from "../../lib/github";
import { relativeTime } from "../../lib/time";
import type { PrFilterId } from "../../types";

interface PullRequestsTabProps {
  pulls: PullRequestInfo[];
  /** More open pull requests exist than the page on screen. */
  hasMore: boolean;
  loading: boolean;
  error: ApiError | null;
  /** The token owner's login; "Review" means requested from them. */
  viewerLogin: string;
  filter: PrFilterId;
  onFilterChange: (filter: PrFilterId) => void;
  repoUrl: string;
}

/** Dot colour token + label for an aggregated check state. */
const CHECK_STATES: Record<CheckState, { color: string; label: string }> = {
  success: { color: "bg-green-dot", label: "Checks passed" },
  failure: { color: "bg-red-dot", label: "Checks failed" },
  pending: { color: "bg-orange-dot", label: "Checks running" },
  // Distinct from "running": a repository may simply have no CI on this branch,
  // and the fixture skips two of the three runs on every pull request.
  none: { color: "bg-fg3", label: "No checks" },
};

export function PullRequestsTab({
  pulls,
  hasMore,
  loading,
  error,
  viewerLogin,
  filter,
  onFilterChange,
  repoUrl,
}: PullRequestsTabProps) {
  const filters: { id: PrFilterId; label: string; match: (pr: PullRequestInfo) => boolean }[] = [
    { id: "open", label: "Open", match: (p) => !p.draft },
    { id: "draft", label: "Draft", match: (p) => p.draft },
    { id: "review", label: "Review", match: (p) => p.requestedReviewers.includes(viewerLogin) },
  ];

  if (loading && pulls.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">Loading pull requests…</p>;
  }

  if (error) {
    return <p className="px-3 py-4 text-xs text-red-dot">{error.message}</p>;
  }

  const active = filters.find((f) => f.id === filter) ?? filters[0];
  const visible = pulls.filter(active.match);

  return (
    <div className="px-1">
      <div className="flex gap-1.5 px-1.5 pb-2 pt-0.5">
        {filters.map((f) => {
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
              {/* Counted over the page that was fetched, not every open PR:
                  REST gives no total without walking all the pages. */}
              <span className="tabular-nums opacity-65">{pulls.filter(f.match).length}</span>
            </button>
          );
        })}
      </div>

      {visible.length === 0 && <p className="px-2 py-3 text-xs text-fg2">Nothing here.</p>}

      {visible.map((pr) => {
        const check = CHECK_STATES[pr.checks];
        return (
          <a
            key={pr.number}
            href={pr.htmlUrl}
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
                {`${pr.author} · ${pr.headBranch} · ${relativeTime(pr.updatedAt)}`}
              </div>
              <div className="mt-[3px] flex items-center gap-2 text-[11px] text-fg2">
                <span className="flex items-center gap-1">
                  <span className={clsx("h-1.5 w-1.5 rounded-full", check.color)} />
                  {check.label}
                </span>
                {pr.requestedReviewers.includes(viewerLogin) && (
                  <span className="font-medium text-accent">Review requested</span>
                )}
              </div>
            </div>
            <span className="flex-none text-[11px] tabular-nums text-fg3">{`#${pr.number}`}</span>
          </a>
        );
      })}

      {hasMore && (
        <a
          href={`${repoUrl}/pulls`}
          target="_blank"
          rel="noreferrer"
          className="mt-1 flex items-center justify-center gap-1.5 rounded-lg border-t-[0.5px] border-sep px-2 py-3 text-[12px] font-medium text-accent no-underline hover:bg-hover"
        >
          View all pull requests on GitHub
          <ExternalLinkIcon width="11" height="11" strokeWidth="1.6" />
        </a>
      )}
    </div>
  );
}
