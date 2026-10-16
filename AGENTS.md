# Clipt — Agent Notes

Windows tray clipboard-history manager. Rust + Tauri v2 backend (`src-tauri/`), vanilla JS/TS frontend (`index.html` + `src/`), bun package manager. Rewritten from the Python/pywebview app in `pywebview old version/`.

## Hard rules

- Never modify `pywebview old version/` (read-only behavioral reference), `dist/` (old build artifacts), or `%APPDATA%\Clipt\` (real user data, ~140 days of history). Legacy migration opens old DBs with `SQLITE_OPEN_READ_ONLY`.
- Don't launch the app from an agent session (`bun run tauri dev`, `cargo run`, built exe) unless the user asks: startup runs one-time legacy migration and live clipboard capture against real user data. Verify with compile + tests only; the user does manual smoke tests.
- Don't `git add`/`git commit` — repo has zero commits and the user manages git. Don't modify the spec or create extra doc files.
- Rust tests must use temp dirs (`tempfile` dev-dep); never point them at the real `%APPDATA%`.
- Keep unchanged: crate `clipt` / lib `clipt_lib`, identifier `com.clipt.app`, vite `outDir: web-dist`, `frontendDist: ../web-dist`, and the icon files.

## Commands (repo root unless noted)

- `bun run build` — `tsc && vite build` → `web-dist/` (Tauri bundles THIS, not `dist/`). tsc is strict: unused TS locals/params fail the build.
- `cargo test` (in `src-tauri/`) — unit tests inline in modules, temp-dir fixtures (all passing).
- `cargo build` (in `src-tauri/`) — compile check. All three commands currently pass.
- `bun run tauri dev` / `bun run tauri build` — user-run only; vite dev server is pinned to port 1420 (strictPort).

## Architecture

- Rust modules (`src-tauri/src/`): `paths.rs`, `settings.rs`, `state.rs` (`AppState`: db Mutex, settings Mutex, paused AtomicBool, dedupe `baseline`), `storage.rs` (+ `storage/media.rs` sha256 blob store, `storage/migrate.rs` legacy import), `clipboard.rs` (Win32 message-only window listener thread), `tray.rs`, `commands.rs`, wired in `lib.rs`.
- App data is `%APPDATA%\Clipt\` resolved from the `APPDATA` env var in `paths.rs` — deliberately NOT Tauri's `app_data_dir()` (identifier-based path is wrong here). The `assetProtocol` scope in `tauri.conf.json` is therefore `$DATA/Clipt/media/**`.
- One SQLite connection behind a Mutex; WAL + 5 s busy timeout. Schema in `storage.rs` `SCHEMA_SQL`; corrupt DB is quarantined to `clipt.db.corrupt-<ts>`, never fatal.
- Timestamps: naive local `%Y-%m-%dT%H:%M:%S%.6f`; a clip's day = date part of its timestamp. Clip kinds: `text` / `image` (content-addressed `media/<sha256>.png`) / `files` (paths only, JSON array).
- Dedupe and self-write suppression via the last-fingerprint `baseline`; only error-reads move the baseline, plain skips don't (see `clipboard.rs` tests).
- UI updates are event-driven (no polling): `clip-added`, `clips-changed`, `watcher-state`.
- Frontend: root `index.html` loads `src/bridge.ts` (font imports), which then dynamically imports `src/app.js` — order matters. `app.js` is plain JS (`CliptApp` class); modal and UI events are bound in `bindEvents`, and all backend calls go through `invoke` directly.
- Tauri v2: JS camelCase command args map to Rust snake_case params (`refId` → `ref_id`); settings serde is camelCase. New window/plugin APIs used from JS need permissions in `src-tauri/capabilities/default.json`; the CSP in `tauri.conf.json` allows self + `asset:` images only.
- Spec drift: AI chat (`ai.rs`, `chat_with_history`, `copy_text`) is NOT implemented — `AiSettings` is reserved for the user's planned AI rework. Pins (`get_pins`/`set_pin`/`remove_pin`) and theme settings postdate the spec. Trust the code over the spec where they differ.
- Windows-only paths are `#[cfg(windows)]`-guarded.

## Docs

- `docs/superpowers/specs/2026-10-16-clipt-tauri-rewrite-design.md` — canonical rewrite spec (source of truth for intended behavior).
- `IMPLEMENTATION-PROMPT.md` — original rewrite brief; the hard rules above come from it.
- `DESIGN.md` — Monolith design-system reference (tokens, glass surfaces, themes); the current UI uses a subset — check `src/styles.css` for what's actually implemented.
- `README.md` — project overview: features, build/run instructions, layout.
- `CONTRIBUTING.md`, `ARCHITECTURE.md`, `SECURITY.md`, `CHANGELOG.md` — standard project docs (setup + checks, module map and flows, security model, release notes).
