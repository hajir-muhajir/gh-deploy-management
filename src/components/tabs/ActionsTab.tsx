import {
  CloseIcon,
  RefreshIcon,
  RunCancelledIcon,
  RunFailureIcon,
  RunSuccessIcon,
} from "../icons";
import { ConfirmButton } from "../ui/ConfirmButton";
import type { ApiError, RunInfo, RunStatus } from "../../lib/github";
import { estimateProgress, runDuration } from "../../lib/progress";
import { formatDuration, relativeTime, secondsBetween } from "../../lib/time";

interface ActionsTabProps {
  runs: RunInfo[];
  loading: boolean;
  error: ApiError | null;
  onCancel: (runId: number) => Promise<void>;
  onRerun: (runId: number) => Promise<void>;
  /** Surfaces failures from cancel/re-run, which are easy to miss otherwise. */
  onActionError: (message: string) => void;
}

function StatusIcon({ status }: { status: RunStatus }) {
  switch (status) {
    case "success":
      return <RunSuccessIcon />;
    case "failure":
      return <RunFailureIcon />;
    case "running":
      return (
        <span className="h-2.5 w-2.5 animate-ghspin rounded-full border-2 border-orange-dot border-r-transparent" />
      );
    case "queued":
      return <span className="h-2.5 w-2.5 rounded-full border-2 border-dotted border-fg3" />;
    case "cancelled":
      return <RunCancelledIcon className="text-fg3" />;
  }
}

/** Right-hand column: elapsed estimate while running, final duration once done. */
function rightLabel(run: RunInfo, progress: number | null): string {
  if (run.status === "queued") return "Queued";
  if (run.status === "running") {
    return progress === null ? formatDuration(secondsBetween(run.startedAt)) : `${progress}%`;
  }
  return formatDuration(runDuration(run));
}

export function ActionsTab({
  runs,
  loading,
  error,
  onCancel,
  onRerun,
  onActionError,
}: ActionsTabProps) {
  if (loading && runs.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">Loading runs…</p>;
  }

  if (error) {
    return <p className="px-3 py-4 text-xs text-red-dot">{error.message}</p>;
  }

  if (runs.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">No workflow runs to show.</p>;
  }

  const send = (action: (id: number) => Promise<void>, id: number) => {
    void action(id).catch((caught) => {
      const message =
        caught && typeof caught === "object" && "message" in caught
          ? String((caught as { message: unknown }).message)
          : String(caught);
      onActionError(message);
    });
  };

  return (
    <div className="px-1">
      {runs.map((run) => {
        const active = run.status === "running" || run.status === "queued";
        const progress = estimateProgress(run, runs);
        return (
          <div
            key={run.id}
            className="flex items-start gap-2.5 rounded-lg px-2 py-[9px] hover:bg-hover"
          >
            <div className="mt-px grid h-4 w-4 flex-none place-items-center">
              <StatusIcon status={run.status} />
            </div>

            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <a
                href={run.htmlUrl}
                target="_blank"
                rel="noreferrer"
                className="truncate text-[13px] font-medium text-fg no-underline hover:underline"
                title={run.title}
              >
                {run.title || run.workflowName}
              </a>
              <div className="truncate text-[11px] text-fg2">
                {`${run.workflowName} #${run.runNumber} · ${run.branch} · ${relativeTime(run.startedAt)}`}
              </div>
              {run.status === "running" && (
                <div className="mt-[5px] h-[3px] overflow-hidden rounded-sm bg-fill">
                  {progress === null ? (
                    // No comparable history yet, so show motion instead of a figure.
                    <div className="h-full w-1/3 animate-pulse rounded-sm bg-orange-dot" />
                  ) : (
                    /* The only inline style in the app: a continuous 0-100% value. */
                    <div
                      className="h-full rounded-sm bg-orange-dot transition-[width] duration-700 ease-linear"
                      style={{ width: `${progress}%` }}
                    />
                  )}
                </div>
              )}
            </div>

            <div className="flex flex-none items-center gap-1">
              <span className="text-[11px] tabular-nums text-fg3">
                {rightLabel(run, progress)}
              </span>
              {active ? (
                <ConfirmButton title="Cancel run" onConfirm={() => send(onCancel, run.id)}>
                  <CloseIcon />
                </ConfirmButton>
              ) : (
                <ConfirmButton title="Re-run" onConfirm={() => send(onRerun, run.id)}>
                  <RefreshIcon width="13" height="13" />
                </ConfirmButton>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
