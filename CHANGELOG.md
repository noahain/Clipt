# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project follows [Semantic Versioning](https://semver.org/). Version 1.x was the original Python/pywebview app; its releases remain available under the v1.x tags.

## [2.0.1] - 2026-10-16

### Changed

- New application icon across the window, tray, and installers.

## [2.0.0] - 2026-10-16

Complete rewrite of Clipt in Rust + Tauri 2. No Python runtime required: a single native Windows application with event-driven clipboard capture. Existing 1.x history and settings are imported automatically on first launch (old data is opened read-only).

### Added

- Image clips and file clips in addition to text (images are stored as content-addressed blobs; copied files are stored as paths, not contents).
- Pins for clips and days, with optional notes.
- Full theming: dark/light modes, background customization (solid color, gradient presets, transparency), and glass/solid/outline surface styles.
- Search scope toggle: search within the selected day or across all history.
- Automatic updates from GitHub Releases: signed installers, a silent check on startup, and a manual check in Settings.
- Corrupt-database quarantine: a damaged database is set aside instead of preventing startup.

### Changed

- Capture is event-driven (native Win32 clipboard listener) instead of 2-second polling, with immediate-repeat suppression built in.
- Storage is a single WAL-mode SQLite database with a content-addressed media store, replacing one database per day; cached images are deleted when no clip references them anymore.
- UI redesigned: custom titlebar, sidebar, glass surfaces, modals, and toast notifications. Day browsing, day labels, copy/delete, and the tray menu (show, pause/resume capture) carry over.
- Launch at login from the tray (quiet startup) is now managed by an in-app toggle.
- The one-time importer migrates 1.x history and settings into the new storage layout.

### Removed

- AI chat (the NVIDIA NIM integration from 1.x) is not part of 2.0.0 - it is planned to return in a future release. Until then, everything stays on your machine.
