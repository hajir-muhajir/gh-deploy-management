/**
 * Small typed wrapper around localStorage for single-value preferences.
 *
 * Both calls swallow failures: an embedded webview can refuse storage outright,
 * and losing a preference must never take the panel down with it.
 */

/** Reads a stored value, falling back unless it is one of the allowed ones. */
export function readEnum<T extends string>(key: string, allowed: readonly T[], fallback: T): T {
  try {
    const stored = localStorage.getItem(key) as T | null;
    if (stored && allowed.includes(stored)) return stored;
  } catch {
    /* localStorage unavailable (private mode / embedded webview) */
  }
  return fallback;
}

export function persist(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    /* ignore */
  }
}
