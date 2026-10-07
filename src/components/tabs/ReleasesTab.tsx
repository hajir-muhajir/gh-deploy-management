import { ChangelogIcon, CopyIcon, ExternalLinkIcon } from "../icons";
import { IconButton } from "../ui/IconButton";
import type { ApiError, ReleaseInfo } from "../../lib/github";
import { relativeTime } from "../../lib/time";

interface ReleasesTabProps {
  releases: ReleaseInfo[];
  /** More releases exist than the page on screen. */
  hasMore: boolean;
  loading: boolean;
  error: ApiError | null;
  repoUrl: string;
  onCopy: (text: string, confirmation: string) => void;
  /** Used to report a release that has no notes to copy. */
  onNotice: (message: string) => void;
}

/**
 * Release titles very often just restate the tag — `v1.84.0` titled
 * "Release v1.84.0" — and the tag already sits in a chip on the row above.
 */
function restatesTag(name: string, tag: string): boolean {
  const strip = (value: string) =>
    value
      .trim()
      .toLowerCase()
      .replace(/^release\s*/, "")
      .replace(/^v/, "");
  return strip(name) === strip(tag);
}

export function ReleasesTab({
  releases,
  hasMore,
  loading,
  error,
  repoUrl,
  onCopy,
  onNotice,
}: ReleasesTabProps) {
  // Drafts are only visible to users with push access and are work in progress.
  const visible = releases.filter((release) => !release.draft);

  if (loading && visible.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">Loading releases…</p>;
  }

  if (error) {
    return <p className="px-3 py-4 text-xs text-red-dot">{error.message}</p>;
  }

  if (visible.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">This repository has no releases.</p>;
  }

  function copyNotes(release: ReleaseInfo) {
    if (!release.body.trim()) {
      onNotice(`${release.tag} has no release notes`);
      return;
    }
    onCopy(release.body, "Changelog copied");
  }

  return (
    <div className="px-1">
      {visible.map((release) => (
        <div
          key={release.id}
          className="flex flex-col gap-[3px] rounded-lg px-2 py-2.5 hover:bg-hover"
        >
          <div className="flex items-center gap-1.5">
            <span className="truncate rounded-[5px] bg-fill px-1.5 py-1 font-mono text-[11.5px] font-semibold leading-none">
              {release.tag}
            </span>
            {release.latest && (
              <span className="flex-none rounded-[5px] bg-green-tint px-1.5 py-[3px] text-[10.5px] font-semibold text-green">
                Latest
              </span>
            )}
            {release.prerelease && (
              <span className="flex-none rounded-[5px] bg-orange-tint px-1.5 py-[3px] text-[10.5px] font-semibold text-orange">
                Pre-release
              </span>
            )}
            <span className="flex-1" />
            <IconButton
              size={24}
              title="Copy tag"
              onClick={() => onCopy(release.tag, `Copied ${release.tag}`)}
            >
              <CopyIcon />
            </IconButton>
            <IconButton size={24} title="Copy changelog" onClick={() => copyNotes(release)}>
              <ChangelogIcon />
            </IconButton>
            <IconButton size={24} title="Open release" href={release.htmlUrl}>
              <ExternalLinkIcon width="12" height="12" strokeWidth="1.6" />
            </IconButton>
          </div>

          {release.name && !restatesTag(release.name, release.tag) && (
            <div className="mt-[3px] text-[13px] font-medium">{release.name}</div>
          )}
          <div className="text-[11px] text-fg2">
            {`${release.author} · ${relativeTime(release.publishedAt)}`}
          </div>
        </div>
      ))}

      {hasMore && (
        <a
          href={`${repoUrl}/releases`}
          target="_blank"
          rel="noreferrer"
          className="mt-1 flex items-center justify-center gap-1.5 rounded-lg border-t-[0.5px] border-sep px-2 py-3 text-[12px] font-medium text-accent no-underline hover:bg-hover"
        >
          View all releases on GitHub
          <ExternalLinkIcon width="11" height="11" strokeWidth="1.6" />
        </a>
      )}
    </div>
  );
}
