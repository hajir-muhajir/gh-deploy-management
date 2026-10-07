import clsx from "clsx";
import { useEffect, useRef, useState } from "react";

/** How long the "Sure?" confirmation stays armed before reverting. */
const CONFIRM_MS = 3000;

interface ConfirmButtonProps {
  title: string;
  onConfirm: () => void;
  disabled?: boolean;
  /** "icon" sits in a row of actions; "primary" is the full-width Run button. */
  variant?: "icon" | "primary";
  children: React.ReactNode;
}

/**
 * Two-click guard for anything that reaches production CI — cancelling a run,
 * re-running one, starting one. The first click only arms the button.
 */
export function ConfirmButton({
  title,
  onConfirm,
  disabled,
  variant = "icon",
  children,
}: ConfirmButtonProps) {
  const [armed, setArmed] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => void (timer.current && clearTimeout(timer.current)), []);

  // Disarm when the button goes away or its action stops being available, so a
  // stale "Sure?" cannot be clicked into a different action.
  useEffect(() => {
    if (disabled) setArmed(false);
  }, [disabled]);

  function onClick() {
    if (armed) {
      if (timer.current) clearTimeout(timer.current);
      setArmed(false);
      onConfirm();
      return;
    }
    setArmed(true);
    timer.current = setTimeout(() => setArmed(false), CONFIRM_MS);
  }

  if (variant === "primary") {
    return (
      <button
        type="button"
        disabled={disabled}
        title={armed ? `${title} — click again to confirm` : title}
        onClick={onClick}
        className={clsx(
          "mt-0.5 flex cursor-pointer items-center justify-center gap-[7px] rounded-lg border-0 py-[9px] text-[13px] font-semibold text-white",
          "disabled:cursor-default disabled:opacity-40",
          armed ? "bg-red-dot" : "bg-accent hover:brightness-110",
        )}
      >
        {armed ? "Click again to confirm" : children}
      </button>
    );
  }

  return (
    <button
      type="button"
      disabled={disabled}
      title={armed ? `${title} — click again to confirm` : title}
      aria-label={title}
      onClick={onClick}
      className={clsx(
        "grid cursor-pointer place-items-center rounded-md border-0 p-0 transition-colors",
        armed
          ? "h-6 w-auto bg-red-dot px-1.5 text-[10px] font-semibold text-white"
          : "h-6 w-6 bg-transparent text-fg2 hover:bg-fill hover:text-fg",
      )}
    >
      {armed ? "Sure?" : children}
    </button>
  );
}
