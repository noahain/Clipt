# Clipt

A clipboard history manager for Windows. Clipt lives in the system tray and records what you copy — text, images, and copied files — into a searchable history organized by day, so you can find and re-copy anything later. Everything stays on your machine, in `%APPDATA%\Clipt\`.

## Features

- Captures copied text, images, and files (file entries store paths, not contents)
- Skips immediate repeats, so copying the same thing twice in a row doesn't add another entry
- Search within the selected day or across all history
- History grouped by day, with editable day labels
- Pin clips or whole days, with an optional note
- Copy any entry back to the clipboard in one click
- Pause and resume capture from the tray
- Optional launch at login
- Dark/light themes with background and surface-style options
- One-time import of history from the original Python version, if present

## Requirements

- Windows (the app uses the native clipboard listener and tray)
- [Rust](https://rustup.rs/) with edition 2024 support (1.85+)
- [bun](https://bun.sh/) for frontend tooling
- Tauri v2 prerequisites: the WebView2 runtime and MSVC build tools — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Building from source

From the repository root:

```sh
bun install
bun run tauri build
```

This produces the executable at `src-tauri/target/release/clipt.exe` and installers (MSI/NSIS) under `src-tauri/target/release/bundle/`.

Launch `clipt.exe` and the window opens; left-click the tray icon any time to bring it back.

## Development

```sh
bun run tauri dev
```

Runs the app with a live-reloading frontend (Vite on port 1420). Backend checks run from `src-tauri/`:

```sh
cargo test
cargo clippy
```

The frontend alone can be built with `bun run build` (tsc + Vite → `web-dist/`).

## Project layout

- `src/` — frontend: `bridge.ts` entry, `app.js` window UI, `styles.css`
- `src-tauri/` — Rust backend: clipboard listener, SQLite storage, tray, IPC commands
- `index.html` — main window shell
- `docs/superpowers/specs/` — the Tauri rewrite design spec
- `AGENTS.md` — working notes for AI agents (architecture, hard rules)
- `DESIGN.md` — UI design-system reference

## Recommended IDE Setup

- [VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
