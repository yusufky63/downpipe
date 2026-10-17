# Architecture

Downpipe is a set of layers that each know as little as possible about the others. The UI does not know how ranges work, the engine does not know about queues, and nothing below the provider layer (planned) will know what YouTube is.

```text
 apps/browser-extension
            |  Native Messaging (stdio)
            v
 crates/native-host  downpipe-native-host
            |  named pipe / Unix socket
            v
 apps/desktop (Tauri + React)        crates/cli (downpipe)
            |  commands + events                |
            v                                   |
 crates/core  downpipe-core                     |
   Manager: queue, state machine,               |
   SQLite catalog, settings, events             |
            |                                   |
            v                                   v
 crates/http-engine  downpipe-http  <-----------+
   Engine -> Job -> workers, SegmentMap, PartFile, meta, RateLimiter
            |
            v
          network / disk
```

## downpipe-http (engine)

One `Engine` owns an HTTP client (connection pool, proxy, user agent) and a `RateLimiter`. `Engine::start(DownloadRequest)` spawns a job and returns a `DownloadHandle` with:

- a `watch` channel of `Progress` snapshots (phase, bytes, speed, connections, segment views, notices),
- `pause()` / `cancel()` (also available as a cloneable `DownloadControl`),
- `wait()` resolving to `Outcome::{Completed, Paused, Canceled}` or a `DownloadError`.

Errors carry a stable `kind()` (`auth_required`, `forbidden`, `not_found`, `throttled`, `resource_changed`, `network`, `checksum`, `io`, ...) so UIs can explain failures without parsing messages.

See [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md).

## downpipe-core (application core)

`Manager` keeps the list of downloads and runs at most `max_concurrent_downloads` of them. Each task has a state:

```text
Queued -> Starting -> Downloading <-> Waiting
                          |  \-> Paused -> Queued
                          |  \-> Failed -> Queued (retry)
                          v
                      Verifying -> Completed
any unfinished state -> Canceled
```

The catalog lives in SQLite (`downpipe.db` in the app data folder): one row per download plus a settings row. It records *which* downloads exist and their last state. *Which bytes are on disk* is recorded only in each download's `.part.meta`, written right after the data is synced. Keeping the byte map out of the database means a crash can never leave the two disagreeing about the file contents.

On start, tasks that were running are put back in the queue and their progress is re-read from the meta files. On a clean shutdown running tasks are paused with a final checkpoint and re-queued, so they continue on the next launch.

**Categories.** When a download is added without a folder, the manager picks one from its category (by the extension of the browser's file name hint or the URL) and marks the record `auto_dir`. When it completes, the real file name is known; if it belongs to another category, the file is renamed into that folder. Records keep the category for filtering. The catalog schema is at version 2 (`category`, `auto_dir`).

**Proxy and user agent** are part of the settings. Changing either builds a new `Engine` (a new connection pool); running downloads keep the one they started with.

**Schedule.** `downpipe_core::schedule` has the pure rules: `due()` decides whether a start or stop time passed between two timer ticks (so a late tick or a computer that slept through the time still acts), and `DoneWatcher` fires once when the number of running and queued downloads drops to zero after having been above zero.

The manager broadcasts `ManagerEvent::{Updated, Removed, Settings}`. The desktop app forwards them to the web view as `downpipe://event`; if the web view falls behind it receives `downpipe://resync` and reloads the list.

Credentials embedded in URLs and custom headers are kept in memory only. The database stores the URL with `user:password@` removed.

## Desktop app

`apps/desktop/src-tauri` exposes commands (`list_downloads`, `add_download`, `pause_download`, `remove_download`, `update_settings`, `pick_folder`, `reveal_download`, ...). The web view has only Tauri's `core:default` permission: folder pickers and opening files run in Rust, and file actions take a download id rather than a path, so the web view cannot open arbitrary files.

The React UI (`apps/desktop/src`) keeps one array of task snapshots and applies events to it (`lib/tasks.ts`); searching, sorting (`lib/sort.ts`) and link patterns (`lib/patterns.ts`) are pure functions with tests.

The browser extension decides what to capture in `apps/browser-extension/src/lib/filters.ts` (file types, excluded sites, minimum size) before anything reaches the app. Its "links on this page" feature injects one self-contained function (`lib/collect.ts`) with `scripting.executeScript` when the user asks, shows the result on an extension page, and sends each chosen link through the same Native Messaging request as a context-menu click; see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

`src-tauri/src/background.rs` runs beside the window: it forwards events to the web view, shows finish notifications when the window is not in front, keeps the tray tooltip current, applies the schedule every 20 seconds and runs the "when done" countdown. `tray.rs` builds the tray menu; closing the window hides it when `close_to_tray` is on. The window is created hidden and shown in `setup`, except when the sign-in item starts Downpipe with `--hidden`.

## Browser bridge

`apps/browser-extension` adds **Download with Downpipe** to link context menus and, when the user switches it on, captures downloads the browser starts. Its service worker sends a small, versioned JSON message through Native Messaging. The browser starts `downpipe-native-host` for it, but only because the host manifest lists the extension's id.

`downpipe-native-host` validates each message (`protocol.rs`), forwards it over a local named pipe or Unix socket (`ipc.rs`) and returns the app's answer. The app serves that endpoint from a Tokio task (`server.rs`, started in `apps/desktop/src-tauri/src/main.rs`), validates the request again and calls `Manager::add` with the referrer as an in-memory `Referer` header. When the app is not running the host answers `app_not_running`, and captured downloads stay with the browser.

The same crate builds both ends of the pipe, so the end-to-end test (`crates/native-host/tests/e2e.rs`) runs the real host binary against the real server and a real `Manager`, downloading from the test server.

Protocol, install locations and troubleshooting: [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

## CLI

`downpipe` talks to the engine directly, without the queue or database. Its resume state is the same `.part.meta` file, so a download started in the CLI can also be resumed by running the command again or with `downpipe resume`.

## Test server

`tools/test-server` serves deterministic content (any byte is a function of its offset and a version number) and injects failures through query parameters. The engine and core integration tests run against it on an ephemeral loopback port.

## Planned layers

- **Media detection in the extension.** Find the manifest or media request behind a page instead of `blob:` URLs and pass candidates over the existing bridge; `page_url` is already part of the protocol for this.
- **Media resolver and providers.** `MediaResolver::resolve(page_url) -> MediaDescriptor` with formats, subtitles and playlists; a `DownloadPlan` of direct HTTP, HLS, DASH and mux steps. yt-dlp runs as an optional external process with argument arrays (never a shell string) and its JSON output is validated before use.
- **Local API.** Loopback-only REST + event stream for a web UI, reusing `Manager`.
