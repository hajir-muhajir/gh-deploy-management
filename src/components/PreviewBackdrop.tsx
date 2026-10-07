import clsx from "clsx";
import type { ReactNode } from "react";
import { TrayIcon } from "./icons";

interface PreviewBackdropProps {
  /** Highlights the tray button while the flyout is open. */
  open: boolean;
  onToggle: () => void;
  badgeCount: number;
  children: ReactNode;
}

/**
 * Desktop stand-in shown only in the browser (`npm run dev`): wallpaper plus a
 * fake taskbar, so the flyout can be seen in context. Never rendered inside Tauri.
 */
export function PreviewBackdrop({ open, onToggle, badgeCount, children }: PreviewBackdropProps) {
  return (
    <div className="relative h-screen min-h-[720px] w-full overflow-hidden wallpaper">
      {children}

      <div className="absolute inset-x-0 bottom-0 flex h-12 items-center justify-end gap-1 border-t-[0.5px] border-sep bg-bar px-2.5 backdrop-blur-[30px]">
        <span className="px-1.5 text-[11px] text-fg2">⌃</span>

        <button
          type="button"
          title="Repositories"
          onClick={onToggle}
          className={clsx(
            "relative grid h-9 w-[34px] cursor-pointer place-items-center rounded-md border-0 text-fg",
            open ? "bg-fill" : "bg-transparent hover:bg-hover",
          )}
        >
          <TrayIcon />
          {badgeCount > 0 && (
            <span className="absolute right-[3px] top-[3px] box-border h-3.5 min-w-3.5 rounded-full bg-red-dot px-[3px] text-center text-[9px] font-bold leading-[14px] text-white">
              {badgeCount}
            </span>
          )}
        </button>

        <div className="flex flex-col items-end px-1.5 text-[11px] leading-[1.35]">
          10:42
          <span>07/10/2026</span>
        </div>
      </div>
    </div>
  );
}
