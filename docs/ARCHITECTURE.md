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

Errors carry a stable `kind()` (`auth_required`, `forbidden`, `not_found`, `throttled`, `resource_changed`, `network`, `checksum`, `io`, `drm_protected`, `live_stream`, ...) so UIs can explain failures without parsing messages.

**Streams.** A `DownloadRequest` with `media` set (`MediaRequest`: HLS or DASH, the chosen tracks, an optional FFmpeg) runs the stream job in `media/` instead of the file job, behind the same `DownloadHandle` and `Progress` (which then carries `media`: quality, chosen tracks, fragments done). `media/hls.rs` and `media/dash.rs` parse manifests into fragment lists and refuse DRM and live streams while parsing; `media/resolve.rs` fetches manifests and turns a choice into a `Plan` of one or two tracks; `media/job.rs` downloads the fragments; `media/mux.rs` finds and runs FFmpeg. `Engine::media_options` lists a stream's qualities without downloading media, for the quality chooser.

See [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md).

## downpipe-core (application core)

`Manager` keeps the list of downloads in queues and runs, per queue, at most that queue's limit of them (`max_concurrent_downloads` for the main queue). Each task has a state:

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

**Categories.** When a download is added without a folder, the manager picks one from its category (by the extension of the browser's file name hint or the URL) and marks the record `auto_dir`. When it completes, the real file name is known; if it belongs to another category, the file is renamed into that folder. Records keep the category for filtering. The catalog schema is at version 3 (`category` and `auto_dir` from version 2, `queue` and `position` from version 3).

**Queues.** `Settings::queues` lists them, the main queue (`main`) always first; ids of new queues are made from their names. Every record has a `queue` and a `position`. The scheduler walks the records in position order and starts a queued download when its queue has a free slot, so queues run side by side and each starts its downloads top to bottom. `move_tasks` reorders the selected downloads inside their queues: each queue keeps the positions it holds in the global order, only which download sits in them changes. `set_queue` moves downloads to the end of another queue; removing a queue in the settings moves its downloads to the main queue.

**Confirmation.** A download added with `confirm` (the browser bridge sets it while `confirm_browser_downloads` is on) is created paused with `awaiting_confirmation` in its live state. `resume_all`, `start_queue` and the schedule leave it alone; `confirm` applies the chosen name, folder and queue and either queues it or keeps it paused. The bridge passes the flag on to the extension in its `accepted` and `tasks` answers, so the popup shows such a download as waiting rather than paused (see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md)). The flag is not stored: after a restart such a download is an ordinary paused one.

**Streams.** A record whose URL is an HLS or DASH manifest has `media` (`MediaInfo`: the kind, the chosen tracks and the quality label), stored as JSON in the catalog's `media` column (schema version 4). `add` sets it from `AddOptions::media` or from an `.m3u8` / `.mpd` path, and files such a download under the Video category. `inspect_media` reads the manifest with the download's own headers and returns its qualities for the confirmation dialog; `confirm` stores the user's choice. When the job resolves a choice (or the best quality, when there was none), the manager stores what it resolved to, so a resume after a restart downloads the same tracks. `Settings::ffmpeg_path` (empty: `ffmpeg` on `PATH`) is looked up when a stream starts. Removing or cancelling a stream download deletes its work folder.

**Download again.** `redownload` resets a completed record and queues it from the same link. With `replace`, the old file is deleted first and the name is kept; otherwise the name is cleared so the engine picks a free one (`name (1).ext`) next to the old file.

**Proxy and user agent** are part of the settings. Changing either builds a new `Engine` (a new connection pool); running downloads keep the one they started with.

**Schedule.** `downpipe_core::schedule` has the pure rules: `due()` decides whether a start or stop time passed between two timer ticks (so a late tick or a computer that slept through the time still acts), and `DoneWatcher` fires once when the number of running and queued downloads drops to zero after having been above zero.

The manager broadcasts `ManagerEvent::{Updated, Removed, Settings}`. The desktop app forwards them to the web view as `downpipe://event`; if the web view falls behind it receives `downpipe://resync` and reloads the list.

Credentials embedded in URLs and custom headers are kept in memory only. The database stores the URL with `user:password@` removed.

## Desktop app

`apps/desktop/src-tauri` exposes commands (`list_downloads`, `add_download`, `pause_download`, `remove_download`, `update_settings`, `pick_folder`, `reveal_download`, ...). The web view has only Tauri's `core:default` permission: folder pickers and opening files run in Rust, and file actions take a download id rather than a path, so the web view cannot open arbitrary files.

The React UI (`apps/desktop/src`) keeps one array of task snapshots and applies events to it (`lib/tasks.ts`); searching, sorting (`lib/sort.ts`), link patterns (`lib/patterns.ts`), the row selection (`lib/selection.ts`), the right-click menu's entries (`lib/menu.ts`) and the keyboard (`lib/shortcuts.ts`) are pure functions with tests. `components/ContextMenu.tsx` draws the menu; the window cancels the web view's own menu outside text fields and the browser's keys (reload, print, find, history, navigation, and the developer tools in release builds).

The browser extension decides what to capture in `apps/browser-extension/src/lib/filters.ts` (file types, excluded sites, minimum size) before anything reaches the app. Its "links on this page" feature injects one self-contained function (`lib/collect.ts`) with `scripting.executeScript` when the user asks, shows the result on an extension page, and sends each chosen link through the same Native Messaging request as a context-menu click; see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

`src-tauri/src/background.rs` runs beside the window: it forwards events to the web view, shows finish notifications when the window is not in front, keeps the tray tooltip and the taskbar progress current, keeps the computer awake while downloads run (`power::KeepAwake`, on a thread of its own because Windows ties the request to the thread that made it), brings the window up when a browser download waits for confirmation, applies the schedule every 20 seconds (to every queue or the one chosen) and runs the "when done" countdown. `clipboard.rs` watches the clipboard while `watch_clipboard` is on: on Windows it reads the text only when the clipboard's change counter moved, elsewhere it compares the text, and it offers one http(s) link whose file type belongs to a category and is not in the list yet (`downpipe://clipboard`). `tray.rs` builds the tray menu; closing the window hides it when `close_to_tray` is on. The window is created hidden and shown in `setup`, except when the sign-in item starts Downpipe with `--hidden`.

## Browser bridge

`apps/browser-extension` adds **Download with Downpipe** to link context menus and, when the user switches it on, captures downloads the browser starts. Its service worker sends a small, versioned JSON message through Native Messaging. The browser starts `downpipe-native-host` for it, but only because the host manifest lists the extension's id.

With **Media on this page** switched on (the optional `webRequest` permission plus access to `http://*/*` and `https://*/*`), the service worker keeps the `Referer` and `Origin` each media, XHR and fetch request sent (`onSendHeaders`), reads the response headers (`onResponseStarted`) and keeps a list per tab in `storage.session`. `lib/media.ts` decides what counts: manifests by type or extension; files when a media element loaded them or a script fetched them whole, filtered by the media types and minimum size of the options page and by the excluded sites. A content script, registered with `scripting.registerContentScripts` only while the switch is on, puts a Download button over playing videos; it asks the service worker for its own tab's list only. Streams go to the app as a `media` message, single media files as an ordinary `download`.

`downpipe-native-host` validates each message (`protocol.rs`), forwards it over a local named pipe or Unix socket (`ipc.rs`) and returns the app's answer. The app serves that endpoint from a Tokio task (`server.rs`, started in `apps/desktop/src-tauri/src/main.rs`), validates the request again and calls `Manager::add` with the referrer as an in-memory `Referer` header. When the app is not running the host answers `app_not_running`, and captured downloads stay with the browser.

The same crate builds both ends of the pipe, so the end-to-end test (`crates/native-host/tests/e2e.rs`) runs the real host binary against the real server and a real `Manager`, downloading from the test server.

Protocol, install locations and troubleshooting: [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

## CLI

`downpipe` talks to the engine directly, without the queue or database. Its resume state is the same `.part.meta` file, so a download started in the CLI can also be resumed by running the command again or with `downpipe resume`.

## Test server

`tools/test-server` serves deterministic content (any byte is a function of its offset and a version number) and injects failures through query parameters. The engine and core integration tests run against it on an ephemeral loopback port.

## Planned layers

- **Site providers.** For sites whose players need site-specific extraction (YouTube among them): an optional, separately updated adapter around yt-dlp that resolves a page into the same `MediaOptions` the stream job already understands, so the quality chooser and FFmpeg merging are shared. yt-dlp runs as an optional external process with argument arrays (never a shell string) and its JSON output is validated before use.
- **Local API.** Loopback-only REST + event stream for a web UI, reusing `Manager`.
