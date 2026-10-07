interface BadgeProps {
  count: number;
}

/** Red pill with a count. Renders nothing when the count is zero. */
export function Badge({ count }: BadgeProps) {
  if (count <= 0) return null;
  return (
    <span className="box-border inline-block h-[15px] min-w-[15px] rounded-lg bg-red-dot px-1 text-center text-[10px] font-semibold leading-[15px] text-white">
      {count}
    </span>
  );
}
