import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";

/**
 * Runs `onShown` each time the tray opens the flyout.
 *
 * The window stays alive while hidden, so without this the panel would show
 * whatever was loaded last time it was open. A timer cannot cover this: WebView2
 * may throttle timers in a hidden window, and the event removes the guesswork.
 */
export function useFlyoutShown(enabled: boolean, onShown: () => void) {
  useEffect(() => {
    if (!enabled) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    void listen("flyout-shown", () => onShown()).then((stop) => {
      // The listener may resolve after this effect was already torn down.
      if (cancelled) stop();
      else unlisten = stop;
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [enabled, onShown]);
}
