# Changelog

All notable changes to Downpipe. Dates are YYYY-MM-DD.

## 0.3.0 — unreleased

The IDM features that save the most time day to day.

### New

- System notification when a download completes or fails while the window is in the background (Settings → App).
- Tray icon with Show, Pause all, Resume all and Quit. Closing the window keeps Downpipe running in the tray, so the browser extension can still hand downloads over. Optional start at sign-in, straight to the tray.
- Search box and sortable Name, Size, Progress, Speed and Left columns; the sort is remembered.
- Drag links or a `.txt` list onto the window to add them, expand patterns such as `file[001-100].zip` or `part[a-f].rar` (up to 1000 links), import a list from a file, and save every link in the list to a text file. Finished downloads can be dragged out of the list into other apps.
- Proxy (`http://`, `https://`, `socks5://`) and user agent settings; downloads started after saving use them.
- Categories: new downloads are sorted into Compressed, Documents, Music, Video, Programs and Disk images folders by file type, and a finished file moves to its real type's folder when the server's name differs from the link. Categories are editable and can be switched off; folders picked by hand are never changed. A type filter next to the search box.
- **Use a new link…** in the details of a paused or failed download, for links that expired. The download continues only if the new link serves the same file (size and ETag); otherwise it fails with "different file" and keeps the partial data.
- Schedule: resume paused and failed downloads at a start time and pause everything at a stop time, on chosen weekdays.
- "When done" in the status bar: quit Downpipe, sleep or shut down once all downloads finish, after a 60-second countdown that can be cancelled or skipped.
- Capture filters in the browser extension (its options page, opened from **Capture filters** in the popup): capture only listed file types (an IDM-like default list of archives, programs, disk images, documents, audio and video, or **Capture all file types**), never capture on excluded sites (`example.com`, `*.example.com`, checked against the download, its redirect target and the page it came from), and leave files under a minimum size to the browser. **Pause capture on this site** in the popup excludes the current site in one click.
- **Download links on this page with Downpipe…** in the page context menu (and **Download selected links…** for a text selection), or **Links on this page…** in the popup, lists the page's http(s) links in a new tab with a text filter, file type chips, **Hide web pages**, select all or none, and sends the chosen links to Downpipe with a result for each. Nothing is sent while Downpipe is not running.
- The extension asks for `activeTab` and `scripting` for this: it reads a page only when you choose the menu item or open the popup on it, once, without content scripts or host permissions.
- Downpipe's own right-click menu on downloads: Open, Show in folder, Resume or Retry, Pause, Download again…, Use a new link…, Move to queue, Queue order, Copy link, Copy file path, Properties and Remove…. It acts on every selected download. The More button on each row, the Menu key and Shift+F10 open it too.
- Selection as in a file manager: click, Ctrl+click, Shift+click, arrow keys, Ctrl+A and Escape. Double click or Enter opens the details, Delete removes, Ctrl+C copies the links, Ctrl+F jumps to the search box and Ctrl+N opens Add link.
- Queues: add queues in **Settings → Queues**, each with its own number of downloads at once, running next to the others. Move downloads between queues and reorder them (Move to top, up, down, to bottom; Ctrl+Home, Ctrl+Up, Ctrl+Down, Ctrl+End) from the menu; inside a queue, downloads start top to bottom. A queue filter next to the search box shows one queue in its order, with Start and Stop buttons. The Add dialog has a queue field, and the schedule can start and stop a single queue.
- **Ask before downloads from the browser start** (on by default, Settings → App): a download sent by the extension waits in a dialog for its file name, folder and queue, with **Start download**, **Download later** and **Cancel**. Nothing is fetched before the answer, and closing the dialog keeps it, paused.
- **Offer to download copied links** (off by default, Settings → App): copying a link to a file type listed under Categories brings up the Add dialog with it. Links already in the list are skipped.
- **Keep the computer awake while downloading** (on by default): sleep waits until downloads stop running; the screen can still turn off.
- Progress of the running downloads on the Downpipe button in the taskbar.
- **Download again…** for a finished download, from the same link: the new copy is saved next to the old one, or replaces it when **Replace the existing file** is ticked.

### Changed

- Download capture takes the default file types only, unless **Capture all file types** is switched on.
- A browser download held by **Ask before downloads from the browser start** is shown in the extension as waiting, not as started or paused: its notification reads "Waiting for your answer in Downpipe", and the popup lists it as **Waiting in Downpipe** until the dialog is answered, then follows it into Downloading or Paused. The bridge's `accepted` and `tasks` answers carry an optional `awaiting_confirmation: true` for this; older extensions ignore it and keep showing Paused.
- Clicking a download selects it; double click or Enter opens its details. The rows' Remove button moved into the menu (and the Delete key).
- The window behaves like an app rather than a web page: the browser's own menu (Back, Reload, Inspect) and keys such as F5, Ctrl+R, Ctrl+P and Ctrl+F do nothing; text fields keep their cut, copy and paste menu. Labels and rows no longer highlight like page text.

### Fixed

- Pausing a download in the moment it was resuming (before the server answered) deleted the partial file from earlier sessions. It is now kept.
- Dialogs moved the keyboard focus back to their first field whenever the list updated, so typing in a later field of the Add dialog jumped while downloads ran.

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
