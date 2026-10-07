import { ChangelogIcon, CopyIcon, ExternalLinkIcon } from "../icons";
import { IconButton } from "../ui/IconButton";
import type { Release } from "../../types";

interface ReleasesTabProps {
  releases: Release[];
  repoUrl: string;
  onCopy: (text: string, confirmation: string) => void;
}

function changelog(release: Release): string {
  return `## ${release.tag} — ${release.name}\n` + release.notes.map((n) => `- ${n}`).join("\n");
}

export function ReleasesTab({ releases, repoUrl, onCopy }: ReleasesTabProps) {
  return (
    <div className="px-1">
      {releases.map((release) => (
        <div
          key={release.tag}
          className="flex flex-col gap-[3px] rounded-lg px-2 py-2.5 hover:bg-hover"
        >
          <div className="flex items-center gap-1.5">
            <span className="rounded-[5px] bg-fill px-1.5 py-1 font-mono text-[11.5px] font-semibold leading-none">
              {release.tag}
            </span>
            {release.latest && (
              <span className="rounded-[5px] bg-green-tint px-1.5 py-[3px] text-[10.5px] font-semibold text-green">
                Latest
              </span>
            )}
            {release.pre && (
              <span className="rounded-[5px] bg-orange-tint px-1.5 py-[3px] text-[10.5px] font-semibold text-orange">
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
            <IconButton
              size={24}
              title="Copy changelog"
              onClick={() => onCopy(changelog(release), "Changelog copied")}
            >
              <ChangelogIcon />
            </IconButton>
            <IconButton
              size={24}
              title="Open release"
              href={`${repoUrl}/releases/tag/${release.tag}`}
            >
              <ExternalLinkIcon width="12" height="12" strokeWidth="1.6" />
            </IconButton>
          </div>
          <div className="mt-[3px] text-[13px] font-medium">{release.name}</div>
          <div className="text-[11px] text-fg2">{`${release.author} · ${release.when}`}</div>
        </div>
      ))}
    </div>
  );
}
