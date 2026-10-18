# Architecture

Downpipe is a set of layers that each know as little as possible about the others. The UI does not know how ranges work, the engine does not know about queues, and nothing below the provider layer knows what YouTube is.

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
            |                    \              |
            |                     v             |
            |        crates/providers  downpipe-providers
            |          video pages (YouTube): yt-dlp as a process
            v                     |             v
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

Errors carry a stable `kind()` (`auth_required`, `forbidden`, `not_found`, `throttled`, `resource_changed`, `expired_link`, `login_required`, `network`, `checksum`, `io`, `drm_protected`, `live_stream`, ...) so UIs can explain failures without parsing messages.

**Streams.** A `DownloadRequest` with `media` set (`MediaRequest`: HLS or DASH, the chosen tracks, an optional FFmpeg) runs the stream job in `media/` instead of the file job, behind the same `DownloadHandle` and `Progress` (which then carries `media`: quality, chosen tracks, fragments done). `media/hls.rs` and `media/dash.rs` parse manifests into fragment lists and refuse DRM and live streams while parsing; `media/resolve.rs` fetches manifests and turns a choice into a `Plan` of one or two tracks; `media/job.rs` downloads the fragments; `media/mux.rs` finds and runs FFmpeg. `Engine::media_options` lists a stream's qualities without downloading media, for the quality chooser.

**Jobs from outside.** `DownloadHandle::spawn` runs a job the engine does not perform itself behind the same handle: the job gets a `JobContext` with the progress channel and the pause and cancel signals, and returns an `Outcome`. The site providers use it, so queues and UIs treat a yt-dlp download like any other. The error kinds `tool_missing`, `unavailable` and `extractor` exist for such jobs.

See [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md).

## downpipe-providers (site providers)

For pages whose video a plain HTTP download cannot reach. YouTube's web player streams over SABR, and its links need PO tokens and answers to JavaScript challenges, so sniffing its requests (what IDM did) no longer works. [yt-dlp](https://github.com/yt-dlp/yt-dlp) does that work; this crate runs it as a separate process, never in-process and never through a shell.

- `pages` recognizes video pages (YouTube's watch, shorts, embed and live pages, `youtu.be`) and turns each into one link per video. The extension's `lib/sites.ts` and the desktop app's `lib/sites.ts` apply the same rules, checked against the same fixture.
- `tools` finds yt-dlp, a JavaScript runtime for YouTube's challenges (Deno 2.3.0 or newer, yt-dlp's default; Node 22 or newer when there is no Deno) and FFmpeg: the path in the settings, else Downpipe's tools folder (`<data folder>/tools`), else `PATH`. `install` fills that folder only when the user presses the button in **Settings → Media**: the latest official release from GitHub, checked against the SHA-256 the project publishes, before it replaces anything.
- `info` runs `yt-dlp -J --no-playlist`, reads only the fields it needs (sizes and types checked, format ids kept only when they are plain tokens) and condenses the formats into the stream chooser's `VideoOption` / `AudioOption` lists: one quality per resolution and frame rate (H.264 before AV1 before VP9, so the result plays everywhere), the sound tracks per language and codec, and "Audio only". Playlists, live and protected videos are refused.
- `job` runs the download: `--ignore-config`, no cookies, `--no-playlist`, a format selector built from the user's choice (the chosen formats first, the closest by height as a fallback), `--merge-output-format mp4/mkv` with the FFmpeg found, and `--progress-template` / `--print` lines that `progress` turns into the engine's `Progress` (two formats of a merged download add up to one size). Pausing ends yt-dlp and every process it started (a job object on Windows, a process group elsewhere); starting again passes `--continue`, which picks up its `.part` files. Cancelling deletes them too. The file name's stem is fixed before the first start, from the confirmation dialog or the page title, and a free one is picked when a finished file already has it.

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

**Confirmation.** A download added with `confirm` (the browser bridge sets it while `confirm_browser_downloads` is on, and the desktop app's Add dialog always sets it for a video page, so its quality is chosen before yt-dlp starts) is created paused with `awaiting_confirmation` in its live state. The Add dialog also sets `from_app`, which the snapshot carries as `from_app` too, so every window can tell where the question belongs: in the main window for what was added there, in the confirmation window for what the browser sent. `resume_all`, `start_queue` and the schedule leave it alone; `confirm` applies the chosen name, folder and queue and either queues it or keeps it paused. The bridge passes the flag on to the extension in its `accepted` and `tasks` answers, so the popup shows such a download as waiting rather than paused (see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md)). The flag is not stored: after a restart such a download is an ordinary paused one.

**Streams.** A record whose URL is an HLS or DASH manifest has `media` (`MediaInfo`: the kind, the chosen tracks and the quality label), stored as JSON in the catalog's `media` column (schema version 4). `add` sets it from `AddOptions::media` or from an `.m3u8` / `.mpd` path, and files such a download under the Video category. `inspect_media` reads the manifest with the download's own headers and returns its qualities for the confirmation dialog; `confirm` stores the user's choice. When the job resolves a choice (or the best quality, when there was none), the manager stores what it resolved to, so a resume after a restart downloads the same tracks. `Settings::ffmpeg_path` (empty: `ffmpeg` on `PATH`) is looked up when a stream starts. Removing or cancelling a stream download deletes its work folder.

**Video pages.** `add` recognizes a video page (`downpipe_providers::pages`), stores it under its canonical link, files it as a video and gives the record `site` (`SiteInfo`: the site, the chosen formats in the stream chooser's `MediaSelection` shape, the quality label), stored as JSON in the catalog's `site` column (schema version 5). The scheduler starts such a record with `downpipe_providers::job` instead of the engine, with the tools found from `yt_dlp_path`, `deno_path` and `ffmpeg_path`, the proxy and the speed limit (as yt-dlp's per-download limit). `inspect_site` asks yt-dlp for the page's formats and title for the confirmation dialog, `confirm` stores the choice, and the formats yt-dlp actually took are stored like a stream's resolved tracks. `site_tools` reports the tools with their versions, and `install_site_tools` downloads yt-dlp and Deno into the tools folder when the user asks.

**Download again.** `redownload` resets a completed record and queues it from the same link. With `replace`, the old file is deleted first and the name is kept; otherwise the name is cleared so the engine picks a free one (`name (1).ext`) next to the old file.

**Proxy and user agent** are part of the settings. Changing either builds a new `Engine` (a new connection pool); running downloads keep the one they started with.

**Schedule.** `downpipe_core::schedule` has the pure rules: `due()` decides whether a start or stop time passed between two timer ticks (so a late tick or a computer that slept through the time still acts), and `DoneWatcher` fires once when the number of running and queued downloads drops to zero after having been above zero.

The manager broadcasts `ManagerEvent::{Updated, Removed, Settings}`. The desktop app forwards them to the web view as `downpipe://event`; if the web view falls behind it receives `downpipe://resync` and reloads the list.

Credentials embedded in URLs and custom headers are kept in memory only. The database stores the URL with `user:password@` removed.

## Desktop app

`apps/desktop/src-tauri` exposes commands (`list_downloads`, `add_download`, `pause_download`, `remove_download`, `update_settings`, `pick_folder`, `reveal_download`, `inspect_site`, `site_tools`, `install_site_tools`, ...). The app has two windows: `main` (`index.html`, `src/App.tsx`) and `confirm` (`confirm.html`, `src/ConfirmWindow.tsx`). Both web views have only Tauri's `core:default` permission: folder pickers and opening files run in Rust, and file actions take a download id rather than a path, so a web view cannot open arbitrary files; showing, hiding and closing windows happen in Rust too.

The React UI (`apps/desktop/src`) keeps one array of task snapshots and applies events to it (`lib/tasks.ts`); searching, sorting (`lib/sort.ts`), link patterns (`lib/patterns.ts`), the row selection (`lib/selection.ts`), the right-click menu's entries (`lib/menu.ts`) and the keyboard (`lib/shortcuts.ts`) are pure functions with tests. `components/ContextMenu.tsx` draws the menu; the window cancels the web view's own menu outside text fields and the browser's keys (reload, print, find, history, navigation, and the developer tools in release builds).

The browser extension decides what to capture in `apps/browser-extension/src/lib/filters.ts` (file types, excluded sites, minimum size) before anything reaches the app. Its "links on this page" feature injects one self-contained function (`lib/collect.ts`) with `scripting.executeScript` when the user asks, shows the result on an extension page, and sends each chosen link through the same Native Messaging request as a context-menu click; see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

`src-tauri/src/background.rs` runs beside the window: it forwards events to the web view, shows finish notifications when the window is not in front, keeps the tray tooltip and the taskbar progress current, keeps the computer awake while downloads run (`power::KeepAwake`, on a thread of its own because Windows ties the request to the thread that made it), opens the confirmation window when a browser download waits for confirmation (and brings the main window up for one added there), applies the schedule every 20 seconds (to every queue or the one chosen) and runs the "when done" countdown. `clipboard.rs` watches the clipboard while `watch_clipboard` is on: on Windows it reads the text only when the clipboard's change counter moved, elsewhere it compares the text, and it offers one http(s) link whose file type belongs to a category, or that `downpipe_providers::pages` recognizes as a video page, and that is not in the list yet, compared the way the list keys its links, so a share link of a listed video is skipped too (`downpipe://clipboard`). Every way of adding a link (the Add dialog, pasting, dropping, a clipboard offer) ends in the Add dialog, which adds video pages with `confirm` and `from_app` so the confirmation dialog asks yt-dlp for their qualities in the main window, where the user is; the dialog then has no "Don't ask again", which is about the browser. `tray.rs` builds the tray menu and renames its items when the language changes (a Linux tray menu cannot be replaced once set); closing the window hides it when `close_to_tray` is on, and otherwise quits. The window is created hidden and shown in `setup`, except when the sign-in item starts Downpipe with `--hidden`.

`confirm.rs` owns the confirmation window, IDM's "Download File Info": downloads the browser sent while `confirm_browser_downloads` is on wait there, one after another, for their name, folder, queue and quality, in `components/ConfirmDialog.tsx` filling a small window with its own taskbar button that comes to the front once per question (on top for a moment, then back in the normal order, with a taskbar flash where Windows keeps the focus elsewhere): one line each for the link, the name (with the size when it is known), the folder, the quality and queue, and the sound track when there is a choice, 520 px wide and as tall as its question needs, neither resizable nor maximizable. The page measures the height (fractional, after every render and once the fonts are in) and calls `fit_confirm_window` with its `devicePixelRatio`; the window is sized in physical pixels from that, rounded up with two spare pixels, so no scrollbar appears at 100, 125 or 150 % display scaling, and only the screen's work area limits it. The main window stays as it was, hidden in the tray or not, and does not ask for them (`lib/tasks.ts` `awaitingIn` splits the waiting downloads by `from_app`). The window is built hidden the first time and kept, hidden, for the next question. The page calls `confirm_window_drawn` once a question is on screen and the window has its height, which shows the window, so it never flashes empty or jumps (a page that does not report within three seconds is shown anyway); the window hides when no browser download waits any more, which the background loop sees in the core's events, so an answer given in either window, or Resume and Remove in the list, ends the question in both. The window's close button asks the page (`downpipe://confirm-close`) to do what Escape does, "Download later" for the download shown. Its Settings links call `open_media_settings`: the main window opens on Settings, Media, and the question is hidden until the main window reports `settings_closed` (or goes to the tray), then shown again with `downpipe://confirm-again` so it is read anew. The pure state machine is unit-tested in `confirm.rs`.

`progress.rs` gives a download started from the confirmation window a progress window of its own, IDM's download dialog: `watch_download` builds `progress-<id>` (the same page, which draws `ProgressWindow.tsx` for such a label) at the confirmation window's place, one step lower and to the right for each window already open, or for a question still waiting. The page follows its download through the core's events and acts on it by id: status, size, speed, time left, resume capability and the segment bar, with Pause or Resume, Cancel and Hide, then Open, Open folder and Close once it is done, or Retry when it failed (`lib/progress.ts` has the texts, with tests). It sizes itself like the confirmation window (`fit_window`), shows once drawn, and names its window after the progress (`set_progress_title`, "45% name"), the only thing the taskbar shows of a download behind other windows. Hiding, closing or cancelling destroys the window rather than keeping it, so many downloads do not keep many web views; the download goes on in the list.

**Translations.** Every visible text lives in `apps/desktop/src/i18n/<language>/` (and `apps/browser-extension/src/i18n/<language>/` for the extension), English in `en/` being the source. A language provides the same structure; the type checker and `catalogs.test.ts` fail when a key, a placeholder or a plural form a language needs is missing. `t()` picks plural forms with `Intl.PluralRules` and fills `{placeholders}`; dates and name sorting use the active language through `Intl`. The app's language comes from `Settings.language` (empty follows the system); the windows switch it before they render. The tray menu, its tooltip and notifications are translated in Rust (`src-tauri/src/i18n.rs`), with the system language read by `sys-locale`. The extension follows the browser's language.

**Duplicate links.** `AddOptions.on_duplicate` decides what `Manager::add` does with a link already in the list, finished or not: `refuse` (the default: `CoreError::Duplicate`, so the app can ask), `add_copy` (a new download whose file gets a free name) or `ask` (the browser bridge: the download is held for confirmation with `duplicate_of` set, and the confirmation window asks IDM's question first). A link with a question already waiting is never held twice.

## Browser bridge

`apps/browser-extension` adds **Download with Downpipe** to link context menus and, when the user switches it on, captures downloads the browser starts. Its service worker sends a small, versioned JSON message through Native Messaging. The browser starts `downpipe-native-host` for it, but only because the host manifest lists the extension's id.

With **Media on this page** switched on (the optional `webRequest` permission plus access to `http://*/*` and `https://*/*`), the service worker keeps the `Referer` and `Origin` each media, XHR and fetch request sent (`onSendHeaders`), reads the response headers (`onResponseStarted`) and keeps a list per tab in `storage.session`. `lib/media.ts` decides what counts: manifests by type or extension; files when a media element loaded them or a script fetched them whole, filtered by the media types and minimum size of the options page and by the excluded sites. A content script, registered with `scripting.registerContentScripts` only while the switch is on, puts a Download button over playing videos; it asks the service worker for its own tab's list only. Streams go to the app as a `media` message, single media files as an ordinary `download`. On a video page (YouTube) the popup, the overlay button and a page context menu item send the page itself as a `page` message, for the site provider; that works without detection, from the popup's `activeTab` grant.

`downpipe-native-host` validates each message (`protocol.rs`), forwards it over a local named pipe or Unix socket (`ipc.rs`) and returns the app's answer. The app serves that endpoint from a Tokio task (`server.rs`, started in `apps/desktop/src-tauri/src/main.rs`), validates the request again and calls `Manager::add` with the referrer as an in-memory `Referer` header. When the app is not running the host answers `app_not_running`, and captured downloads stay with the browser. The app registers the host next to its own executable for the user's browsers at every start; an AppImage registers a copy in its data folder instead, because its own files live in a mount that moves with every start.

The same crate builds both ends of the pipe, so the end-to-end test (`crates/native-host/tests/e2e.rs`) runs the real host binary against the real server and a real `Manager`, downloading from the test server.

Protocol, install locations and troubleshooting: [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

## CLI

`downpipe` talks to the engine directly, without the queue or database. Its resume state is the same `.part.meta` file, so a download started in the CLI can also be resumed by running the command again or with `downpipe resume`.

`downpipe get` refuses a video page (`downpipe_providers::pages`) with a pointer to the desktop app, before any request: downloading it would save the page's HTML, and running yt-dlp without the app's quality chooser would take its best quality, which can be a very large 4K file.

## Test server

`tools/test-server` serves deterministic content (any byte is a function of its offset and a version number) and injects failures through query parameters. The engine and core integration tests run against it on an ephemeral loopback port.

## Planned layers

- **More site providers.** Other sites yt-dlp supports are one entry each in `pages` and `lib/sites.ts`, once their terms and extraction have been checked.
- **Local API.** Loopback-only REST + event stream for a web UI, reusing `Manager`.
