# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A Windows system-tray flyout for watching GitHub Actions, releases and pull requests across a set of
tracked repositories. Tauri 2 shell (Rust) + React 19 / TypeScript / Tailwind 4 webview.

## Commands

```bash
npm run tauri dev            # the real app — starts Vite on :1420 and the Rust shell
npm run dev                  # webview only, in a browser (see "Browser preview" below)
npm run build                # tsc && vite build — the frontend typecheck lives here
npm run tauri build          # installers into src-tauri/target/release/bundle/

cd src-tauri && cargo test                                   # all Rust tests
cd src-tauri && cargo test --lib                             # unit tests only (no network)
cd src-tauri && cargo test --test github_api                 # live API integration tests
cd src-tauri && cargo test --test github_api -- the_test_name # a single test
cd src-tauri && cargo fmt && cargo clippy
```

There is no JS test runner and no ESLint config — `npm run build` (tsc) is the only frontend check.

`src-tauri/tests/github_api.rs` hits the real GitHub API. It reads a PAT from `$GITHUB_TOKEN` or a
gitignored `.env.local` at the repo root, and every test **skips itself** when neither is present —
so a green run means nothing unless the token is there. The fixture repo is pinned in that file
(`FIXTURE`); it depends on that repo's actual workflow layout.

Tray and app icons are generated, not hand-edited:

```bash
node scripts/make-tray-icons.mjs                     # -> src-tauri/icons/tray/**
node scripts/make-app-icon.mjs && npx tauri icon app-icon.png
```

## Architecture

### The token boundary

The GitHub PAT lives in the Windows Credential Manager ([secrets.rs](src-tauri/src/secrets.rs)) and
**never reaches the webview**. Every GitHub call goes Rust-side: `client_from_keychain()` reads the
token per invocation, builds a `GitHub` client, makes the request, and returns a derived model. The
frontend only ever sees `TokenInfo` (login, scopes, rate limit) — not the token. Keep it that way:
don't add a command that returns the token, and don't move an API call into TypeScript.

### Rust layering (`src-tauri/src/`)

- [github/](src-tauri/src/github/) — the split mirrors the direction data flows: `raw` is what GitHub
  sends, `models` is what the webview receives, `status`/`dispatch` hold the derivations in between,
  and `client` is the only part that does I/O. `error` keeps credential material out of messages.
- [commands.rs](src-tauri/src/commands.rs) — the entire `invoke` surface, one thin wrapper per client
  method. Adding a command means: method on `GitHub` → wrapper here → entry in the
  `generate_handler!` list in [lib.rs](src-tauri/src/lib.rs) → typed wrapper in
  [src/lib/github.ts](src/lib/github.ts).
- [flyout.rs](src-tauri/src/flyout.rs) — window show/hide/position. Two subtleties: blur hides the
  panel, so a tray click needs the `REOPEN_GRACE` window to not instantly re-open what the blur just
  closed; and the webview stays alive while hidden, so showing emits `flyout-shown` to trigger a
  refresh (timers may be throttled in a hidden WebView2).
- Plugin order in `Builder` is load-bearing: `tauri-plugin-single-instance` must stay **first**. Its
  callback has to be in place before anything else runs, or a second launch builds its own tray icon
  before being told it is a duplicate (that was the "tray icon multiplies" bug). The callback calls
  `flyout::show` — deliberately not `toggle`, whose `just_hidden` grace period exists for the
  tray-click gesture, not for a shortcut click.
- Autostart: the registry entry (`is_enabled()`) is the source of truth for the toggle; localStorage
  holds only `ghdm.autostart.decided`, the marker that the first-run default has been applied, so
  turning the toggle off is never silently undone on the next launch.
- [tray/](src-tauri/src/tray/) — `resolve()` picks the one winning badge (failing > review > running >
  idle), unit-tested. Windows has no tray badge API, so every count is a separate baked PNG embedded
  via `include_bytes!` — a missing asset is a compile error.

### Badge priority is decided in Rust, deliberately

The frontend sends **raw counts** via `set_tray_state(failing, review, running)`; Rust decides which
badge wins. [FlyoutPanel.tsx](src/components/flyout/FlyoutPanel.tsx) duplicates that priority only to
colour the browser preview's fake taskbar. If you change the priority, change both.

### Frontend shape

One hook per GitHub resource (`useRuns`, `usePulls`, `useReleases`, `useWorkflows`, `useRepos`,
`useDispatchable`), each with the same contract: `(fullName, enabled)` → `{ data, loading, error,
refresh }`, an `alive` flag to drop stale responses, and `syncedAt` set **only on success** so a
failed refresh can't make stale data look fresh. [FlyoutPanel.tsx](src/components/flyout/FlyoutPanel.tsx)
is the single stateful orchestrator; everything under `components/` below it is presentational.

Refresh tiers are intentional, because requests are expensive against the 5000/hour budget:

- auto-refresh timer ticks **only** runs + pulls (what the badge is made of), default 5m
- opening the flyout (`flyout-shown`) refreshes everything
- `useDispatchable` is gated on the Run tab being open — it costs one request per active workflow
  because the trigger and inputs exist only in the workflow YAML, parsed in `github/dispatch.rs`
- `list_pulls` costs one request per PR for check state (the Checks API 403s for these tokens and the
  Statuses API reports a false `pending`, so head-commit workflow runs are the only truthful source)

Preferences are `localStorage` only, `ghdm.*` keys, via [lib/prefs.ts](src/lib/prefs.ts) — nothing
user-facing is persisted Rust-side except the token.

### Browser preview

`npm run dev` renders the panel in a plain browser. `useIsTauri` switches the app into preview mode:
`PreviewBackdrop` draws a fake Windows desktop and taskbar around the panel, and `lib/github.ts`'s
`call()` rejects with `kind: "unavailable"` instead of an opaque `invoke` TypeError. This is the fast
loop for UI work; anything touching GitHub needs `npm run tauri dev`.

### Styling

Tailwind 4, no config file — the design tokens are CSS custom properties in
[src/index.css](src/index.css), switched by `data-theme` (light/dark), `data-accent`
(blue/indigo/graphite) and `data-glass` (off inside Tauri, on in the preview), then exposed to
Tailwind through `@theme inline`. Use the semantic utilities (`bg-panel`, `text-fg2`, `border-sep`)
rather than raw colours.

## Conventions

- `src-tauri/rustfmt.toml` sets `max_width = 110` and `use_small_heuristics = "Max"` on purpose —
  short struct literals, match arms and calls stay on one line. Run `cargo fmt`, don't fight it.
- Comments in this codebase explain *why* — a non-obvious API behaviour, a Windows quirk, a cost
  trade-off — not what the line does. Match that when adding code.
- `owner/name` (`fullName`) is the stable repo identity everywhere; workflow identity is `path`, not
  `name` (names are not unique within a repo).
