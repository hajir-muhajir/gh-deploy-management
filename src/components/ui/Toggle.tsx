import clsx from "clsx";

interface ToggleProps {
  checked: boolean;
  onChange: () => void;
  label: string;
}

/** 38x22 switch used by Settings rows and boolean workflow inputs. */
export function Toggle({ checked, onChange, label }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={onChange}
      className={clsx(
        "relative h-[22px] w-[38px] flex-none cursor-pointer rounded-full border-0 p-0 transition-colors duration-200",
        checked ? "bg-green-dot" : "bg-fill",
      )}
    >
      <span
        className={clsx(
          "absolute top-0.5 h-[18px] w-[18px] rounded-full bg-white shadow-knob transition-[left] duration-200",
          checked ? "left-[18px]" : "left-0.5",
        )}
      />
    </button>
  );
}
