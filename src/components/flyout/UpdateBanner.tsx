import { IconButton } from "../ui/IconButton";
import type { UseUpdater } from "../../hooks/useUpdater";

interface UpdateBannerProps {
  updater: UseUpdater;
}

/**
 * One thin row offering the new version. Nothing is downloaded until the user
 * asks: the panel is a glanceable tray flyout, so an update must never take it
 * over or restart the app unannounced.
 */
export function UpdateBanner({ updater }: UpdateBannerProps) {
  const { version, status, install, dismiss } = updater;
  if (!version || (status !== "available" && status !== "downloading")) return null;

  const downloading = status === "downloading";

  return (
    <div className="mx-3 mb-2 flex items-center gap-2 rounded-lg bg-accent-tint px-2.5 py-1.5 text-xs">
      <span className="flex-1 truncate text-fg">
        Version {version} is available
      </span>

      <button
        type="button"
        onClick={install}
        disabled={downloading}
        className="cursor-pointer rounded-md border-0 bg-accent px-2 py-1 text-xs font-medium text-white transition-opacity disabled:cursor-default disabled:opacity-60"
      >
        {downloading ? "Updating…" : "Update"}
      </button>

      {!downloading && (
        <IconButton title="Dismiss update" onClick={dismiss} size={24}>
          <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
            <path
              d="M4 4l8 8M12 4l-8 8"
              stroke="currentColor"
              strokeWidth="1.5"
              strokeLinecap="round"
              fill="none"
            />
          </svg>
        </IconButton>
      )}
    </div>
  );
}
