const MINUTE = 60;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** Seconds between two ISO timestamps, or from an ISO timestamp until now. */
export function secondsBetween(fromIso: string, toIso?: string): number {
  const from = Date.parse(fromIso);
  const to = toIso ? Date.parse(toIso) : Date.now();
  if (Number.isNaN(from) || Number.isNaN(to)) return 0;
  return Math.max(0, Math.round((to - from) / 1000));
}

/**
 * "just now" / "12m ago" / "4d ago", falling back to a short date past a week —
 * the same shape the design used, but derived from real timestamps.
 */
export function relativeTime(iso: string): string {
  const seconds = secondsBetween(iso);
  if (seconds < 45) return "just now";
  if (seconds < HOUR) return `${Math.round(seconds / MINUTE)}m ago`;
  if (seconds < DAY) return `${Math.round(seconds / HOUR)}h ago`;
  if (seconds < 7 * DAY) return `${Math.round(seconds / DAY)}d ago`;

  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return "";
  const sameYear = date.getFullYear() === new Date().getFullYear();
  return date.toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(sameYear ? {} : { year: "numeric" }),
  });
}

/** "41s" / "3m 41s" / "1h 04m" — matches the `3m 41s` style in the design. */
export function formatDuration(seconds: number): string {
  if (seconds < MINUTE) return `${seconds}s`;
  if (seconds < HOUR) {
    const minutes = Math.floor(seconds / MINUTE);
    return `${minutes}m ${String(seconds % MINUTE).padStart(2, "0")}s`;
  }
  const hours = Math.floor(seconds / HOUR);
  const minutes = Math.floor((seconds % HOUR) / MINUTE);
  return `${hours}h ${String(minutes).padStart(2, "0")}m`;
}
