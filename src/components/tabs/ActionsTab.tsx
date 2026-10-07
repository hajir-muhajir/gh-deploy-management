import {
  CloseIcon,
  RefreshIcon,
  RunCancelledIcon,
  RunFailureIcon,
  RunSuccessIcon,
} from "../icons";
import { IconButton } from "../ui/IconButton";
import type { WorkflowRun } from "../../types";

interface ActionsTabProps {
  runs: WorkflowRun[];
  onCancel: (runNumber: number) => void;
  onRerun: (runNumber: number) => void;
}

function StatusIcon({ status }: { status: WorkflowRun["status"] }) {
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

function rightLabel(run: WorkflowRun): string {
  if (run.status === "running") return `${Math.round(run.pct ?? 0)}%`;
  if (run.status === "queued") return "Queued";
  return run.dur;
}

export function ActionsTab({ runs, onCancel, onRerun }: ActionsTabProps) {
  if (runs.length === 0) {
    return <p className="px-3 py-4 text-xs text-fg2">No workflows selected.</p>;
  }

  return (
    <div className="px-1">
      {runs.map((run) => {
        const active = run.status === "running" || run.status === "queued";
        return (
          <div
            key={run.n}
            className="flex items-start gap-2.5 rounded-lg px-2 py-[9px] hover:bg-hover"
          >
            <div className="mt-px grid h-4 w-4 flex-none place-items-center">
              <StatusIcon status={run.status} />
            </div>

            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <div className="truncate text-[13px] font-medium">{run.title}</div>
              <div className="truncate text-[11px] text-fg2">
                {`${run.wf} #${run.n} · ${run.branch} · ${run.ago}`}
              </div>
              {run.status === "running" && (
                <div className="mt-[5px] h-[3px] overflow-hidden rounded-sm bg-fill">
                  {/* The only inline style in the app: a continuous 0-100% value. */}
                  <div
                    className="h-full rounded-sm bg-orange-dot transition-[width] duration-700 ease-linear"
                    style={{ width: `${Math.round(run.pct ?? 0)}%` }}
                  />
                </div>
              )}
            </div>

            <div className="flex flex-none items-center gap-1">
              <span className="text-[11px] tabular-nums text-fg3">{rightLabel(run)}</span>
              {active ? (
                <IconButton size={24} title="Cancel run" onClick={() => onCancel(run.n)}>
                  <CloseIcon />
                </IconButton>
              ) : (
                <IconButton size={24} title="Re-run" onClick={() => onRerun(run.n)}>
                  <RefreshIcon width="13" height="13" />
                </IconButton>
              )}
            </div>
          </div>
        );
      })}
    </div>
  );
}
