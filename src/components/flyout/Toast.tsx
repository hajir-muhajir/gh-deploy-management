import clsx from "clsx";

interface ToastProps {
  message: string | null;
}

export function Toast({ message }: ToastProps) {
  return (
    <div
      className={clsx(
        "pointer-events-none absolute bottom-[46px] left-1/2 -translate-x-1/2 whitespace-nowrap rounded-full bg-fg px-3 py-[7px] text-xs font-medium text-solid transition-opacity duration-200",
        message ? "opacity-100" : "opacity-0",
      )}
    >
      {message ?? ""}
    </div>
  );
}
