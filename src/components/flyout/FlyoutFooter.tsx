import { useEffect, useState } from "react";
import { relativeTime } from "../../lib/time";

/** How often the relative label is recomputed; `relativeTime` resolves to minutes. */
const TICK_MS = 30_000;

interface FlyoutFooterProps {
  /** Null before a repository has been picked. */
  repoUrl: string | null;
  /** ISO time of the newest successful fetch, or null if none has landed. */
  syncedAt: string | null;
}

export function FlyoutFooter({ repoUrl, syncedAt }: FlyoutFooterProps) {
  // The label ages on its own, so it needs a render even when nothing refetches.
  // The ticker lives here rather than in FlyoutPanel so no other component is
  // re-rendered twice a minute for a line of text.
  const [, setTick] = useState(0);
  useEffect(() => {
    const id = setInterval(() => setTick((n) => n + 1), TICK_MS);
    return () => clearInterval(id);
  }, []);

  const label = !repoUrl
    ? "Not connected"
    : syncedAt
      ? `Synced ${relativeTime(syncedAt)}`
      : "Not synced yet";

  return (
    <div className="flex items-center justify-between border-t-[0.5px] border-sep px-4 py-[9px] text-[11px] text-fg2">
      <span>{label}</span>
      {repoUrl && (
        <a
          href={repoUrl}
          target="_blank"
          rel="noreferrer"
          className="font-medium text-accent no-underline hover:underline"
        >
          Open on GitHub
        </a>
      )}
    </div>
  );
}
