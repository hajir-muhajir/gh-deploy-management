export interface Repo {
  owner: string;
  name: string;
  /** `owner/name` — the stable identity used as a key everywhere. */
  fullName: string;
  /** Two-letter initials shown in the avatar tile. */
  ini: string;
  defaultBranch: string;
  private: boolean;
  archived: boolean;
  canPush: boolean;
  /** Whether the repo appears in the header switcher. */
  visible: boolean;
}

export type TabId = "releases" | "run" | "actions" | "prs";
export type ViewId = TabId | "settings";
export type PrFilterId = "open" | "draft" | "review";

/** Mirrors the tray badge: red for failing runs, orange for reviews. */
export type BadgeTone = "failing" | "review";
export interface TrayBadge {
  count: number;
  tone: BadgeTone;
}

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type AccentId = "blue" | "indigo" | "graphite";

/** How often runs and pull requests are refetched on their own. */
export type RefreshInterval = "off" | "1m" | "5m" | "15m";
