import type { RunInfo } from "./github";
import { secondsBetween } from "./time";

/** Never claim a run is finished — only GitHub can tell us that. */
const CEILING = 95;

/** Wall-clock seconds a finished run took. */
export function runDuration(run: RunInfo): number {
  return secondsBetween(run.startedAt, run.updatedAt);
}

function median(values: number[]): number | null {
  if (values.length === 0) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 0 ? (sorted[middle - 1] + sorted[middle]) / 2 : sorted[middle];
}

/**
 * GitHub exposes no progress figure for a running workflow, so this estimates
 * one from how long the same workflow usually takes. `history` is the same run
 * list already on screen, so this costs no extra request.
 *
 * Returns null when there is nothing to compare against — the caller should
 * then show an indeterminate bar rather than invent a number.
 */
export function estimateProgress(run: RunInfo, history: RunInfo[]): number | null {
  if (run.status !== "running") return null;

  const past = history
    .filter((other) => other.workflowId === run.workflowId && other.id !== run.id)
    .filter((other) => other.status === "success" || other.status === "failure")
    .map(runDuration)
    .filter((seconds) => seconds > 0);

  const typical = median(past);
  if (!typical) return null;

  const elapsed = secondsBetween(run.startedAt);
  return Math.min(CEILING, Math.round((elapsed / typical) * 100));
}
