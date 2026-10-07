import clsx from "clsx";
import type { ReactNode } from "react";

interface IconButtonProps {
  title: string;
  onClick?: () => void;
  /** Renders an anchor instead of a button. */
  href?: string;
  children: ReactNode;
  /** 28px is used in the header, 24px inside list rows. */
  size?: 24 | 28;
  active?: boolean;
  className?: string;
}

export function IconButton({
  title,
  onClick,
  href,
  children,
  size = 28,
  active = false,
  className,
}: IconButtonProps) {
  const classes = clsx(
    "grid cursor-pointer place-items-center rounded-md border-0 p-0 transition-colors",
    size === 28 ? "h-7 w-7 rounded-[7px]" : "h-6 w-6",
    active ? "bg-accent-tint text-accent" : "bg-transparent text-fg2 hover:bg-hover hover:text-fg",
    className,
  );

  if (href) {
    return (
      <a href={href} target="_blank" rel="noreferrer" title={title} className={classes}>
        {children}
      </a>
    );
  }

  return (
    <button type="button" title={title} aria-label={title} onClick={onClick} className={classes}>
      {children}
    </button>
  );
}
