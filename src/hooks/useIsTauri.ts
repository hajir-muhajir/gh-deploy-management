/** True when the frontend runs inside the Tauri webview rather than a plain browser. */
export function useIsTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}
