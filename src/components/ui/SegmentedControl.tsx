import clsx from "clsx";
import type { ReactNode } from "react";

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  /** Rendered after the label, e.g. a count badge. */
  trailing?: ReactNode;
}

interface SegmentedControlProps<T extends string> {
  options: SegmentOption<T>[];
  /** Compared against each option's value; a non-matching value renders no active segment. */
  value: string;
  onChange: (value: T) => void;
  className?: string;
}

/** Pill-track segmented control: tab bar, choice inputs and appearance pickers. */
export function SegmentedControl<T extends string>({
  options,
  value,
  onChange,
  className,
}: SegmentedControlProps<T>) {
  return (
    <div className={clsx("flex rounded-lg bg-fill p-0.5", className)}>
      {options.map((option) => {
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            onClick={() => onChange(option.value)}
            className={clsx(
              "flex flex-1 cursor-pointer items-center justify-center gap-[5px] rounded-md border-0 px-0 py-[5px] text-xs transition-colors duration-150",
              active
                ? "bg-seg font-semibold text-fg shadow-seg"
                : "bg-transparent font-medium text-fg2",
            )}
          >
            {option.label}
            {option.trailing}
          </button>
        );
      })}
    </div>
  );
}
