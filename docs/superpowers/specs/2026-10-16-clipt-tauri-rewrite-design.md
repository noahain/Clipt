# Clipt — Rust + Tauri v2 Rewrite

**Date:** 2026-10-16
**Status:** Awaiting user review (rev 2 — all vibe-tester gap findings resolved)

## Goal

Rewrite Clipt (a Windows system-tray clipboard history manager with AI chat) from Python (pywebview + pystray) to **Rust + Tauri v2**, reworking the backend logic where the old design was flawed, while keeping the same product role:

- Runs in the tray, watches the clipboard, organizes clips by day.
- Chat with an AI about a day's clipboard history.
- Existing user data carries over via one-time migration.
- A new UI is being sketched by the user and will replace the interim UI later.

## Scope

### In scope

- Rust backend rewrite: clipboard watcher, storage, tray, window/single-instance behavior, settings, AI chat port, IPC commands.
- New storage format (single SQLite DB) + one-time migration of existing per-day data.
- Capture of **text, images, and copied files** (file paths only, no content duplication).
- Clip **deletion** and basic **search**.
- Interim UI: the old HTML/CSS/JS ported into the Tauri frontend with minimal adaptation.
- Fixing known bugs from the old implementation (list below).

### Out of scope (deferred, user-owned)

- **AI chat rework** (streaming, context strategy, UI). Ported *as-is* behaviorally; the user will plan improvements. Module is isolated so it can be replaced wholesale.
- **New UI design** — user is sketching it; interim UI is disposable.
- Linux/macOS support — Windows-first, but code kept portable where cheap.
- Installers, auto-updater, data encryption.

## Architecture

Tauri v2 app, vanilla TypeScript + Vite frontend, bun package manager.

### Rust modules (`src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `main.rs` / `lib.rs` | Entry point, builder wiring, plugin registration, command registration, app state |
| `paths.rs` | Resolve app data locations |
| `settings.rs` | Load/save `settings.json`; one-time import from legacy `.env` |
| `storage.rs` | SQLite open (WAL), schema, all queries (days/clips/labels/delete/search) |
| `storage/migrate.rs` | One-time import of legacy `Days\` folders |
| `storage/media.rs` | Content-addressed blob store for images; reference-counted GC |
| `clipboard.rs` | Win32 clipboard listener thread, format classification, dedupe, save, event emit |
| `ai.rs` | Frozen port of the old AI handler (NVIDIA NIM, non-streaming) |
| `tray.rs` | Native tray icon + menu, pause/resume toggle, exit |
| `commands.rs` | All Tauri commands exposed to the frontend |
| `state.rs` | Shared app state (storage handle, settings, paused flag) |

### Concurrency & DB contract

- **Single `rusqlite::Connection` behind a `Mutex`** in app state (watcher thread and commands share it). WAL mode, `busy_timeout` 5 s.
- If `clipt.db` is corrupt/unopenable at launch: rename it to `clipt.db.corrupt-<timestamp>`, create a fresh DB, log the incident. Never refuse to launch.
- The `paused` flag is an `AtomicBool` in state (single writer at a time; tray and IPC both set it).

### Key crates (verify versions at implementation)

- `rusqlite` (bundled SQLite), `serde`/`serde_json`, `sha2`, `image` (DIB→PNG), `reqwest` (AI), `clipboard-win`, `windows` (clipboard listener events), `chrono`, `anyhow`/`thiserror`.
- Tauri plugins: `tauri-plugin-single-instance`, `tauri-plugin-autostart`, `tauri-plugin-log`.
- Frontend: `marked` (bundled locally, replaces CDN), `@tauri-apps/api`.

## Data design

### App data root: `%APPDATA%\Clipt\` (unchanged, so legacy files stay put)

| Path | Role |
|---|---|
| `clipt.db` | New single SQLite DB (WAL mode) |
| `media\<sha256>.png` | Image blobs, content-addressed (identical images stored once) |
| `settings.json` | App-managed settings (below) |
| `logs\` | Rolling log files (1 MB × 5 files, oldest rotated out) |
| `Days\` | **Legacy. Imported once, then never touched by the new app.** |
| `.env` | **Legacy. Imported once into settings.json, then ignored.** |
| `assets\`, `webview_cache\` | Legacy leftovers; ignored |

### Schema

```sql
CREATE TABLE days (
  date       TEXT PRIMARY KEY,           -- YYYY-MM-DD
  label      TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL,
  updated_at TEXT
);

CREATE TABLE clips (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  date       TEXT NOT NULL REFERENCES days(date),
  timestamp  TEXT NOT NULL,
  kind       TEXT NOT NULL CHECK (kind IN ('text','image','files')),
  text       TEXT,                       -- kind=text
  blob_hash  TEXT,                       -- kind=image, filename in media\
  file_paths TEXT                        -- kind=files, JSON array of absolute paths
);
CREATE INDEX idx_clips_date_ts ON clips(date, timestamp);

CREATE TABLE meta (key TEXT PRIMARY KEY, value TEXT);  -- schema_version, legacy_import_done
```

- **Timestamps:** naive local time, pinned format `%Y-%m-%dT%H:%M:%S%.6f` (always 6 fractional digits). Legacy imported values keep their original format (mixed-format lexicographic ordering is still correct). Never offset-aware.
- The clip's **day = date part of its timestamp** at save time.
- Day rows are created on first clip save or label edit. `get_days` sorts by `date` descending; `clip_count` is `COUNT(*)` per day.

### Clip capture rules

- **Kind priority among *enabled* kinds only:** files (`CF_HDROP`) > image (`CF_DIB`/PNG) > plain text. If the highest-priority enabled format is absent, fall through to the next enabled kind; if no enabled kind is present, nothing is saved. (The `capture.*` toggles gate participation; disabled kinds are invisible to classification.)
- **Dedupe:** compare against the most recent stored clip — identical text, identical image hash, or the same file set. Works across midnight (old app lost its dedupe baseline at day rollover).
- **File-list comparison:** normalized (trim trailing separators, case-insensitive, dedupe within list) and **order-insensitive** (set comparison).
- Skip whitespace-only text; skip empty file lists.
- **No text size cap** (parity with old app; a giant copy may later blow the AI context — same `"Error: ..."` failure mode as before, acceptable until the user's AI plan).
- **Image gate:** applies to the **converted PNG byte size**; images over `capture.maxImageBytes` (default 25 MB) are skipped and logged.
- **Clipboard locked/read failure:** retry 3× with 100 ms spacing; on final failure log once and update the dedupe baseline (so the failure isn't retried forever). Plain skips (dup/whitespace) do **not** move the baseline.
- **Pause:** initial state `false` at launch (not persisted). While paused, capture events are ignored entirely. On resume the current clipboard becomes the baseline (content copied while paused is not saved — parity with old behavior). Setting the same value again is a no-op (no re-baseline).
- **Self-write suppression:** after any app-initiated clipboard write (`copy_clip` / `copy_text`), the baseline is updated to the written content — copying a clip back never creates a duplicate clip or spurious event.

### Migration (one-time, on first launch; idempotent)

1. If `Days\` exists and `meta.legacy_import_done` is unset: for each folder named `YYYY-MM-DD`, open its `history.db` **read-only** (`SQLITE_OPEN_READ_ONLY`, no sidecar files ever created) and read `metadata.json` if present.
2. Insert day rows for every valid-named folder (label/created/updated from `metadata.json` when present; otherwise `label=''`, `created_at` = date at midnight). ✓ matches old sidebar behavior of listing all folders.
3. Insert clips with an **existence check** (`date` + `timestamp` + `text` match → skip), so re-running the migration is safe and never duplicates. New clips appended to `Days\` by the old app after a migration are picked up if the flag is cleared and migration re-runs.
4. Missing/empty/corrupt per-day files: log and continue; the day keeps an empty clip list.
5. Set `meta.legacy_import_done=1`. **Nothing legacy is ever modified or deleted.**

Known limitation: the old app and new app should not run concurrently long-term; normal flow is to switch at migration time.

## App behaviors

- **Tray (native Tauri menu):** items "Open Clipt" (default), dynamic "Pause"/"Resume", separator, "Exit". Left-click shows the window; right-click opens the menu. Replaces pystray *and* the old frameless webview tray menu.
- **Close button → hide to tray** (native `CloseRequested` interception).
- **Minimize:** normal taskbar behavior (parity — old app didn't intercept minimize).
- **Single instance:** `tauri-plugin-single-instance`; a second launch **always shows + focuses the existing window, regardless of its arguments** (`--startup` only affects the initial creation of a process). Replaces the TCP port 47200 wake hack.
- **`--startup` flag:** launch hidden (tray only). Used by the autostart entry.
- **Autostart:** `launchAtLogin` in settings, managed by `tauri-plugin-autostart` with the `--startup` arg; toggling it applies to the OS entry **immediately**.
- **Window:** 1200×800, min 900×600, title "Clipt" (parity).
- **Logging:** rolling file log in `logs\` via `tauri-plugin-log` (1 MB × 5 files), console in dev.

## IPC surface (Tauri commands + events)

Commands:

| Command | Args | Returns |
|---|---|---|
| `get_days` | — | `Vec<DayInfo { date, label, clip_count, created_at, updated_at }>` sorted by date desc |
| `get_day_data` | `date` | `{ clips: Vec<Clip>, metadata: DayInfo }`; nonexistent date → empty clips + `{date, label:''}`, count 0, nulls |
| `update_day_label` | `date, label` | `bool` — validates `YYYY-MM-DD` (invalid → `false`), creates the day row if missing (parity), trims label, truncates to 200 chars; `false` only on DB error |
| `search_clips` | `query, limit?` | `LIKE %q%` on `text` of text-kind clips only, ASCII case-insensitive, `%`/`_`/`\` escaped (`ESCAPE '\'`); default limit 100, max 500; ordered `timestamp` desc; empty/whitespace query → `[]` |
| `delete_clip` | `id` | `bool` (deletes row; blob file deleted synchronously when no other clip references the hash; file-deletion failure logged, orphan blob harmless) |
| `copy_clip` | `id` | `bool` — text/image(existing blob)/files. Missing media blob → `false`; file paths: writes the existing subset, `false` if none exist (partial write = `true`); clipboard write fails after retries → `false`. Always updates watcher baseline on success |
| `copy_text` | `text` | `bool` — arbitrary text (AI code-block copy buttons); same baseline update |
| `chat_with_history` | `date, query, session_history?` | `String` — frozen behavior |
| `get_settings` / `update_settings` | partial | `update_settings` merges (not replaces), validates/clamps values (`maxImageBytes ≥ 1024`), applies `launchAtLogin` immediately, returns the effective full settings |
| `get_watcher_state` / `set_paused` | `paused` | `{ paused: bool }` |

`Clip` payload: `{ id, date, timestamp, kind, text?, media_path?, file_paths? }` (`date` included so events are self-contained; `media_path` is the absolute blob path for `convertFileSrc`).

Events (push, no polling):

- `clip-added { clip }` — UI: if `clip.date` === visible day, append the card; always refetch the days list (single cheap query; sidebar holds no state worth preserving).
- `clips-changed { date }` — emitted on `delete_clip`; UI refetches that day + the days list.
- `watcher-state { paused }` — UI updates status dot/text.

Media display: frontend converts `media_path` via `convertFileSrc`; asset protocol scope covers `%APPDATA%\Clipt\media`. Missing blob file → UI shows a `[image missing]` fallback card.

## Settings (`settings.json`)

```json
{
  "ai": { "apiKey": "", "model": "qwen/qwen3.5-122b-a10b", "name": "Clipt", "persona": "<default below>" },
  "capture": { "text": true, "images": true, "files": true, "maxImageBytes": 26214400 },
  "launchAtLogin": false
}
```

- One-time import from legacy `.env`: `NVIDIA_API_KEY`→`ai.apiKey`, `AI_MODEL`→`ai.model`, `AI_NAME`→`ai.name`, `AI_PERSONA`→`ai.persona` (verbatim — the persona's clip-formatting instructions must survive exactly). Imported once (flag in `meta`); afterwards `.env` is ignored, documented behavior.
- Default persona (when `.env` had none): `"You are Clipt, a helpful clipboard history assistant."` — matches old fallback.
- **Corrupt/unparseable `settings.json`:** rename to `settings.json.bak-<timestamp>`, recreate defaults, log a warning; never refuse to launch.

## AI chat (frozen port)

Behavior preserved exactly, in one isolated module:

- OpenAI-compatible request to `https://integrate.api.nvidia.com/v1/chat/completions` via `reqwest`.
- Defaults: model `qwen/qwen3.5-122b-a10b`, temperature 0.4, top_p 0.8, max_tokens 16384, **non-streaming**.
- System prompt: persona + name + numbered day context; last 10 session messages appended; errors returned as `"Error: ..."` strings (parity).
- **Context line formats per kind** (the only new-clip-kind accommodation): text → `N. [%I:%M %p] <content>` (unchanged); image → `N. [%I:%M %p] [image]`; files → `N. [%I:%M %p] [files: <basename>, ...]`.
- Marked in code as frozen pending the user's own AI plan.

## Interim UI strategy

Old UI runs almost verbatim, becomes fully disposable when the new sketch lands:

- `index.html`, `app.js`, `styles.css` ported from `ui/` into the Vite frontend; template demo (greet) removed.
- A thin bridge shim assigns `window.pywebview = { api: { …invoke() wrappers } }` so `app.js` needs almost no changes; `marked` bundled via npm and exposed as the same global. Shim mappings: `write_clipboard(text)`→`copy_text`, `get_today_context`→`get_day_data(today)`.
- Minimal additions for the new clip kinds: image cards render a thumbnail (with `[image missing]` fallback); file cards render file basenames; copy buttons re-copy via `copy_clip`.
- Status dot/text wired to real `watcher-state`; the 5 s full-re-render polling is removed in favor of `clip-added` / `clips-changed` events.

## Known bugs fixed by this rewrite (user will verify after)

1. 5 s polling re-rendered everything, losing scroll/expand state → event-driven updates.
2. Clipboard writes via hidden-PowerShell `Set-Clipboard` (fragile quoting) → native writes.
3. Dedupe baseline lost at midnight → last-clip comparison at save time.
4. Tray menu hardcoded monitor size → native tray menu.
5. Close-to-tray deadlock workaround (timer + `return False`) → native close interception.
6. `marked.js` from CDN (breaks offline) → bundled.
7. Sidebar clip counts stale until manual refresh → updated via events.
8. Sidebar status text hardcoded → real watcher state.
9. TCP single-instance/wake hack (port conflicts) → single-instance plugin.
10. UI wait-loop for pywebview bridge readiness → not needed with `invoke`.
11. No clip deletion / no search (README promised search but never shipped it) → both added.
12. Copying a clip back re-saved it as a duplicate → self-write suppression.

## Testing plan

- **Rust tests:** storage CRUD, dedupe rules (incl. cross-midnight, file-list set comparison, image hash), media GC (reference counting, orphan tolerance), migration against synthetic legacy fixtures **including degenerate days** (0-byte DB, no metadata.json), migration idempotency (double-run), settings `.env` import + corrupt-file recovery, search (escaping/limits/ordering), delete, label validation.
- **Classification tests:** enabled-kind priority/fall-through and baseline-update rules as pure functions with raw inputs.
- Clipboard listener itself + tray: manual smoke.
- **E2E manual checklist (parity):** copy text/image/files → appear correctly; tray open/pause/exit; close-hides; second launch focuses; `--startup` hidden; label edit persists; chat answers; copy-back for each kind (no duplicate clip); delete; search.
- All automated suites run via test-engineer before completion claims.

## Phases

1. Core data: paths, settings + `.env` import, schema, media store, migration (+tests).
2. Clipboard watcher: listener, classification, dedupe, saving, events, pause (+tests).
3. App shell: tray, close-hide, single instance, `--startup`, autostart, logging.
4. AI port + `copy_*`, `delete_clip`, `search_clips` commands.
5. Interim UI port + bridge shim + media cards + event wiring.
6. Verification: cargo test, frontend build, dev smoke test, parity checklist; then user tests and reports remaining bugs.

## Open items

- User's bug report (after testing the rewrite) will be triaged into fixes.
- New UI replaces the interim one when the sketch is ready.
- AI chat rework is planned by the user separately.
