import { useCallback, useEffect, useRef, useState } from "react";

const DISMISS_MS = 1700;

export function useToast() {
  const [message, setMessage] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const showToast = useCallback((text: string) => {
    if (timer.current) clearTimeout(timer.current);
    setMessage(text);
    timer.current = setTimeout(() => setMessage(null), DISMISS_MS);
  }, []);

  /** Copy to clipboard (best effort) and confirm with a toast. */
  const copy = useCallback(
    (text: string, confirmation: string) => {
      void navigator.clipboard?.writeText(text).catch(() => {});
      showToast(confirmation);
    },
    [showToast],
  );

  useEffect(() => () => void (timer.current && clearTimeout(timer.current)), []);

  return { message, showToast, copy };
}
