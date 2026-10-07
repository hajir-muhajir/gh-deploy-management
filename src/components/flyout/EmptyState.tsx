interface EmptyStateProps {
  title: string;
  body: string;
  actionLabel: string;
  onAction: () => void;
}

/** Shown when there is no repository to display — no token yet, or none picked. */
export function EmptyState({ title, body, actionLabel, onAction }: EmptyStateProps) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-8 text-center">
      <p className="text-[13px] font-semibold">{title}</p>
      <p className="text-[11px] leading-relaxed text-fg2">{body}</p>
      <button
        type="button"
        onClick={onAction}
        className="mt-1 cursor-pointer rounded-lg border-0 bg-accent px-3 py-1.5 text-xs font-semibold text-white hover:brightness-110"
      >
        {actionLabel}
      </button>
    </div>
  );
}
