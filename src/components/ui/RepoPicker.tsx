import clsx from "clsx";
import { useMemo, useState } from "react";
import type { RepoInfo } from "../../lib/github";

const MAX_RESULTS = 6;
const FULL_NAME = /^[\w.-]+\/[\w.-]+$/;

interface RepoPickerProps {
  /** Search corpus — every repo the token can reach. */
  available: RepoInfo[];
  /** Full names already tracked; filtered out of the suggestions. */
  trackedKeys: Set<string>;
  onAdd: (fullName: string) => Promise<void>;
  disabled?: boolean;
}

/**
 * Search-and-pick field for the repository list. The account can have hundreds
 * of repositories, so they are filtered locally here rather than all rendered.
 */
export function RepoPicker({ available, trackedKeys, onAdd, disabled }: RepoPickerProps) {
  const [query, setQuery] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const trimmed = query.trim();

  const matches = useMemo(() => {
    if (!trimmed) return [];
    const needle = trimmed.toLowerCase();
    return available
      .filter(
        (repo) =>
          repo.fullName.toLowerCase().includes(needle) &&
          !trackedKeys.has(repo.fullName.toLowerCase()),
      )
      .slice(0, MAX_RESULTS);
  }, [available, trackedKeys, trimmed]);

  // Nothing matched locally, but it still looks like a repo path — let the user
  // try it anyway; it may be a public repo outside their account.
  const canTryDirect = matches.length === 0 && FULL_NAME.test(trimmed);

  async function submit(fullName: string) {
    setBusy(true);
    setError(null);
    try {
      await onAdd(fullName);
      setQuery("");
    } catch (caught) {
      const message =
        caught && typeof caught === "object" && "message" in caught
          ? String((caught as { message: unknown }).message)
          : String(caught);
      setError(message);
    } finally {
      setBusy(false);
    }
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key !== "Enter" || busy) return;
    if (matches.length > 0) void submit(matches[0].fullName);
    else if (canTryDirect) void submit(trimmed);
  }

  return (
    <div className="flex flex-col border-t-[0.5px] border-sep">
      <div className="flex gap-1.5 px-3 py-2">
        <input
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setError(null);
          }}
          onKeyDown={onKeyDown}
          placeholder="Search or owner/repository"
          spellCheck={false}
          disabled={disabled}
          className="min-w-0 flex-1 rounded-md border-0 bg-fill px-2 py-1.5 font-mono text-xs text-fg outline-none disabled:opacity-50"
        />
        <button
          type="button"
          disabled={disabled || busy || (!canTryDirect && matches.length === 0)}
          onClick={() => void submit(matches.length > 0 ? matches[0].fullName : trimmed)}
          className="cursor-pointer rounded-md border-0 bg-fill px-3 text-xs font-semibold text-accent disabled:cursor-default disabled:opacity-40"
        >
          {busy ? "…" : "Add"}
        </button>
      </div>

      {matches.length > 0 && (
        <ul className="flex flex-col pb-1.5 pl-3 pr-3">
          {matches.map((repo) => (
            <li key={repo.fullName}>
              <button
                type="button"
                disabled={busy}
                onClick={() => void submit(repo.fullName)}
                className="flex w-full cursor-pointer items-center gap-2 rounded-md border-0 bg-transparent px-2 py-1.5 text-left hover:bg-hover"
              >
                <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-fg">
                  {repo.fullName}
                </span>
                {repo.private && <span className="flex-none text-[10px] text-fg3">private</span>}
                {repo.archived && <span className="flex-none text-[10px] text-fg3">archived</span>}
              </button>
            </li>
          ))}
        </ul>
      )}

      {trimmed && matches.length === 0 && (
        <p
          className={clsx(
            "px-3 pb-2 text-[11px]",
            canTryDirect ? "text-fg2" : "text-fg3",
          )}
        >
          {canTryDirect
            ? "Not in your account — press Add to look it up on GitHub"
            : "No match. Type owner/repository to look one up directly."}
        </p>
      )}

      {error && <p className="px-3 pb-2 text-[11px] text-red-dot">{error}</p>}
    </div>
  );
}
