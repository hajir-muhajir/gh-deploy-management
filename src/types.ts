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

export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type AccentId = "blue" | "indigo" | "graphite";
