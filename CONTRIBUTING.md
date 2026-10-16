# Contributing

This is a personal project and there is no issue tracker set up yet — until the repository is hosted somewhere with one, raise problems or suggestions directly with the maintainer.

## Prerequisites

- Windows (the app is built around the Win32 clipboard listener and tray)
- [Rust](https://rustup.rs/) 1.85+ (the crate uses edition 2024)
- [bun](https://bun.sh/) for the frontend toolchain
- Tauri v2's Windows prerequisites: the WebView2 runtime and MSVC build tools — see the [Tauri prerequisites guide](https://v2.tauri.app/start/prerequisites/)

## Setup

Clone the repository, then from its root:

```sh
bun install
```

## Build and run

From the repository root:

```sh
bun run tauri dev      # development: app with a live-reloading frontend (Vite on port 1420)
bun run tauri build    # production build; artifacts under src-tauri/target/release/
```

## Tests and checks

From `src-tauri/`:

```sh
cargo test             # full Rust suite (tests live inline in modules)
cargo clippy           # lint check
```

Run a single test by name filter, for example:

```sh
cargo test baseline_moves_only_on_save
```

From the repository root, `bun run build` runs `tsc` (strict) and the Vite build — it is the frontend typecheck.

There is no CI in this repository; run the checks locally before sharing changes.

## Conventions

- Commit messages: `type(scope): summary` — `feat`, `fix`, `docs`, `chore`, `refactor`, `test`.
- Rust tests must use temp directories (`tempfile`) and never touch the real `%APPDATA%\Clipt` data.
- No formatter is enforced beyond what `cargo clippy` checks; match the surrounding style.

## Layout and architecture

See [README.md](README.md) for the project layout and [ARCHITECTURE.md](ARCHITECTURE.md) for how the pieces fit together.
