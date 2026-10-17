# Changelog

All notable changes to Downpipe. Dates are YYYY-MM-DD.

## 0.2.0 — unreleased

Second milestone: a link clicked in the browser downloads in the desktop app.

### New

- Browser extension for Chrome, Edge (Manifest V3) and Firefox 128+: **Download with Downpipe** in the link context menu, optional capture of downloads the browser starts, and a toolbar popup that shows whether the app and the bridge are reachable and the last five results.
- Download capture is off by default and asks for the `downloads` permission only when switched on. If Downpipe is not running, the browser keeps the download. Private windows, `blob:` and `data:` downloads are never captured.
- `downpipe-native-host`: Native Messaging host with a versioned, size-limited message schema that refuses cookies, non-web URLs and unknown fields. Its `install`, `uninstall`, `status` and `manifest` commands register it per user (registry key on Windows, manifest folders on macOS and Linux) with the allowed extension ids listed explicitly.
- The desktop app serves the bridge on a per-user named pipe (Windows) or Unix socket, never a network port, and validates every request again before queueing it.
- The desktop installers include `downpipe-native-host`, and the app registers it for the current user at every start: a missing, unreadable or outdated registration, or one for another copy of the host, is rewritten for Chrome, Edge, Chromium and Firefox, keeping extension ids added by hand. Machine-wide registrations are never touched. No `install` command is needed after installing Downpipe.
- The Windows uninstallers (NSIS and MSI) remove that registration for the uninstalling user, but only where it still starts the host being removed: a registration that a build from source or another install has taken over is kept. Updates keep it. `downpipe-native-host uninstall --only-if-host <path>` does the same by hand.
- **Settings → Browser bridge** shows the registration for each browser, can register on demand, and turns the start-up registration off for people who manage it themselves.
- Starting Downpipe while it is already running brings the open window forward instead of opening a second copy that could not serve the bridge.
- The test server has an index page at `/` that links every sample, for trying the extension by hand.
- The extension confirms every hand-over with a system notification ("Downloading in Downpipe"), which can be switched off in the popup. Problems, such as Downpipe being closed, stay on screen until dismissed.
- The popup's Downloads list shows the live state of what this browser sent (Queued, Downloading with percent and speed, Done, Failed), refreshed every second while it is open, through a new `tasks` message that asks the app only about those downloads.

### Changed

- `CoreError::code()` gives the stable error code the desktop UI and the bridge share.
- The extension popup's "bridge not installed" and version-mismatch hints point at starting Downpipe first; `downpipe-native-host install` is only needed for builds from source.
- `pnpm desktop` and `tauri build` build `downpipe-native-host` first (`pnpm --filter @downpipe/desktop sidecar`).
- Clicking a download opens its details in a dialog over the list instead of a panel under it. Live speed, time left and connections appear only while it runs; finished downloads show where they were saved and when.

### Fixed

- The extension popup no longer collapses into a narrow column when Chrome first opens it.

## 0.1.0 — unreleased

First milestone: a reliable HTTP download from start to finish, including pause, restart and crash recovery.

### New

- HTTP engine with multiple HTTP/1.1 connections per download, adaptive connection count (2 to 8 by default) and work stealing between segments.
- Resume across pauses, app restarts and crashes, validated with `If-Range`, ETag and size; a changed remote file restarts cleanly.
- Retries with backoff and jitter, `Retry-After` support, fewer connections when a server rate limits.
- Optional SHA-256 / SHA-512 verification before the file is renamed into place.
- Safe file names from `Content-Disposition` or the URL; existing files are never overwritten.
- Global speed limit, adjustable during downloads.
- Persistent queue with a concurrency limit, duplicate detection and automatic continuation after restart.
- Desktop app: download list with per-segment progress, details panel, add-links dialog with multi-link paste, settings, light and dark themes.
- `downpipe` CLI with `get`, `resume` and `probe`.
- Fault-injection test server for tests and benchmarks.
