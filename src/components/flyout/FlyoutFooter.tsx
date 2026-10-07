interface FlyoutFooterProps {
  repoUrl: string;
}

export function FlyoutFooter({ repoUrl }: FlyoutFooterProps) {
  return (
    <div className="flex items-center justify-between border-t-[0.5px] border-sep px-4 py-[9px] text-[11px] text-fg2">
      <span>Synced just now</span>
      <a
        href={repoUrl}
        target="_blank"
        rel="noreferrer"
        className="font-medium text-accent no-underline hover:underline"
      >
        Open on GitHub
      </a>
    </div>
  );
}
