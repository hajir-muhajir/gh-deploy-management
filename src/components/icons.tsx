import type { SVGProps } from "react";

type IconProps = SVGProps<SVGSVGElement>;

const stroked = {
  fill: "none",
  stroke: "currentColor",
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

export function ChevronDownIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 10 10" width="10" height="10" strokeWidth="1.6" {...stroked} {...props}>
      <path d="M2 3.5l3 3 3-3" />
    </svg>
  );
}

export function RefreshIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="15" height="15" strokeWidth="1.5" {...stroked} {...props}>
      <path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9" />
      <path d="M13.5 2.5v3h-3" />
    </svg>
  );
}

export function SettingsIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="15"
      height="15"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      {...props}
    >
      <path d="M2.5 5h11M2.5 11h11" />
      <circle cx="6" cy="5" r="1.8" className="fill-solid" />
      <circle cx="10.5" cy="11" r="1.8" className="fill-solid" />
    </svg>
  );
}

export function ExternalLinkIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="14" height="14" strokeWidth="1.5" {...stroked} {...props}>
      <path d="M9.5 2.5h4v4" />
      <path d="M13.5 2.5L7.5 8.5" />
      <path d="M11.5 9.5v4h-9v-9h4" />
    </svg>
  );
}

export function CheckIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 12 12" width="12" height="12" strokeWidth="1.8" {...stroked} {...props}>
      <path d="M2.5 6.2l2.4 2.3 4.6-5" />
    </svg>
  );
}

export function CopyIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="13"
      height="13"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinejoin="round"
      {...props}
    >
      <rect x="5" y="5" width="8.5" height="8.5" rx="2" />
      <path d="M2.5 10.5v-7a1 1 0 0 1 1-1h7" />
    </svg>
  );
}

export function ChangelogIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="13"
      height="13"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      {...props}
    >
      <path d="M3 4h10M3 8h10M3 12h6" />
    </svg>
  );
}

export function RunSuccessIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" {...props}>
      <circle cx="8" cy="8" r="7" className="fill-green-dot" />
      <path
        d="M5 8.2l2 2 4-4.4"
        fill="none"
        stroke="#fff"
        strokeWidth="1.6"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

export function RunFailureIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" {...props}>
      <circle cx="8" cy="8" r="7" className="fill-red-dot" />
      <path
        d="M5.7 5.7l4.6 4.6M10.3 5.7l-4.6 4.6"
        stroke="#fff"
        strokeWidth="1.6"
        strokeLinecap="round"
      />
    </svg>
  );
}

export function RunCancelledIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      {...props}
    >
      <circle cx="8" cy="8" r="6.25" />
      <path d="M4 12L12 4" />
    </svg>
  );
}

export function CloseIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 12 12"
      width="11"
      height="11"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      {...props}
    >
      <path d="M3 3l6 6M9 3L3 9" />
    </svg>
  );
}

export function PullRequestIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" strokeWidth="1.5" {...stroked} {...props}>
      <circle cx="4" cy="3.5" r="1.6" />
      <circle cx="4" cy="12.5" r="1.6" />
      <circle cx="12" cy="12.5" r="1.6" />
      <path d="M4 5.1v5.8" />
      <path d="M12 10.9V6.5a2 2 0 0 0-2-2H7" />
      <path d="M8.5 3L7 4.5 8.5 6" />
    </svg>
  );
}

export function EyeIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 16 16" width="15" height="15" fill="none" stroke="currentColor" strokeWidth="1.4" {...props}>
      <path d="M1.5 8s2.5-4.5 6.5-4.5S14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" />
      <circle cx="8" cy="8" r="2" />
    </svg>
  );
}

export function PlayIcon(props: IconProps) {
  return (
    <svg viewBox="0 0 12 12" width="11" height="11" fill="currentColor" {...props}>
      <path d="M3 1.8v8.4L10 6z" />
    </svg>
  );
}

export function TrayIcon(props: IconProps) {
  return (
    <svg
      viewBox="0 0 16 16"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      {...props}
    >
      <circle cx="4" cy="3.5" r="1.6" />
      <circle cx="4" cy="12.5" r="1.6" />
      <circle cx="12" cy="6" r="1.6" />
      <path d="M4 5.1v5.8M12 7.6c0 2.4-2 3-6.4 4" />
    </svg>
  );
}
