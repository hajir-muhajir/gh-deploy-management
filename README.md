# GitHub Deploy Management

A Windows system-tray flyout for keeping an eye on GitHub Actions, releases and pull requests across
the repositories you care about. Click the tray icon, see what's failing, re-run or cancel a run,
start a `workflow_dispatch` job — then click away and it's gone.

Built with [Tauri 2](https://tauri.app) (Rust) and React 19 + TypeScript + Tailwind 4.

## Features

- **Tray badge** that shows the one thing that matters most: failed runs (red), pull requests
  awaiting *your* review (orange), or a workflow in flight (dot).
- **Actions** — recent workflow runs, with cancel and re-run.
- **Run** — start any `workflow_dispatch` workflow, with its real input form read from the YAML.
- **Pull requests** — open PRs with their check state, filterable by draft / awaiting review.
- **Releases** — the latest releases with their notes.
- **Multiple repositories**, switchable from the header; hide the ones you don't want in the list.
- **Light/dark + accent themes**, following Windows by default.
- **Single instance** — opening the shortcut again brings up the panel you already have running
  instead of starting a second copy.
- **Starts with Windows** by default, switchable from Settings.

Your personal access token is stored in the Windows Credential Manager and never leaves the Rust
side of the app.

## Requirements

- Node.js 18+
- Rust toolchain (stable) and the [Tauri prerequisites](https://tauri.app/start/prerequisites/)
- A GitHub personal access token
  - **Classic**: needs the `repo` and `workflow` scopes
  - **Fine-grained**: needs *Contents: read*, *Pull requests: read*, *Metadata: read*, and
    *Actions: read & write* (write is required to dispatch, cancel and re-run)

## Getting started

```bash
npm install
npm run tauri dev
```

The app starts hidden in the tray. Click the icon to open the panel, go to **Settings**, paste your
token, then pick the repositories you want to watch.

### Frontend-only preview

```bash
npm run dev
```

Opens the panel in a browser inside a mock Windows desktop, for fast UI iteration. GitHub calls are
unavailable there — use `npm run tauri dev` for anything that touches the API.

## Building

```bash
npm run tauri build
```

Installers land in `src-tauri/target/release/bundle/`.

## Tests

```bash
cd src-tauri
cargo test --lib             # unit tests, no network
cargo test --test github_api # integration tests against the real GitHub API
```

The integration tests need a real token in `$GITHUB_TOKEN` or in a gitignored `.env.local` at the
repo root. Without one, each test skips itself — so check the output rather than just the exit code.

## Icons

The tray and app icons are generated from the design source rather than hand-edited:

```bash
node scripts/make-tray-icons.mjs                      # tray badge set
node scripts/make-app-icon.mjs && npx tauri icon app-icon.png
```

## Recommended IDE setup

[VS Code](https://code.visualstudio.com/) +
[Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) +
[rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
