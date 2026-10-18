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
   (crates/crawl  downpipe-crawl: site grabber, RSS/Atom; used by core)
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

A `DownloadRequest` may carry a `limiter` of its own: a download then draws from its bucket and the engine's, so it runs at whichever limit is lower. The core keeps one per download and changes its rate while the download runs.

Errors carry a stable `kind()` (`auth_required`, `forbidden`, `not_found`, `throttled`, `resource_changed`, `expired_link`, `login_required`, `network`, `checksum`, `io`, `drm_protected`, `live_stream`, ...) so UIs can explain failures without parsing messages.

**Streams.** A `DownloadRequest` with `media` set (`MediaRequest`: HLS or DASH, the chosen tracks, an optional FFmpeg) runs the stream job in `media/` instead of the file job, behind the same `DownloadHandle` and `Progress` (which then carries `media`: quality, chosen tracks, fragments done). `media/hls.rs` and `media/dash.rs` parse manifests into fragment lists and refuse DRM and live streams while parsing; `media/resolve.rs` fetches manifests and turns a choice into a `Plan` of one or two tracks; `media/job.rs` downloads the fragments; `media/mux.rs` finds and runs FFmpeg. `Engine::media_options` lists a stream's qualities without downloading media, for the quality chooser.

**Jobs from outside.** `DownloadHandle::spawn` runs a job the engine does not perform itself behind the same handle: the job gets a `JobContext` with the progress channel and the pause and cancel signals, and returns an `Outcome`. The site providers use it, so queues and UIs treat a yt-dlp download like any other. The error kinds `tool_missing`, `unavailable` and `extractor` exist for such jobs.

**File hosts.** `hosts` turns a recognized file host's share link into the file's address before each start (Google Drive, including its "can't scan for viruses" confirmation page); the resume data stays filed under the link the user gave. A file that is not shared publicly fails with `private_file`.

See [DOWNLOAD_ENGINE.md](DOWNLOAD_ENGINE.md).

## downpipe-crawl (finding files on the web)

Reads web pages and feeds with a plain HTTP client (the manager's, so proxy and user agent apply; no cookie store), never more than a few MiB per page and with a time limit per request.

- `html` finds the links of a page with a small tolerant tokenizer: `href` / `src` of anchors, frames and media elements, `<base>`, `<meta http-equiv="refresh">`; comments and the contents of `<script>`, `<style>` and `<textarea>` are skipped.
- `robots` reads `robots.txt` (RFC 9309): the group naming Downpipe, else `*`; the longest matching rule wins, `Allow` wins a tie; `*` and `$` wildcards; `Crawl-delay`. A missing file allows everything, a server error disallows everything.
- `site::grab` is the site grabber: breadth first from the start page (depth 1), following only pages of the start's site (`www.` aside) unless told otherwise, up to a depth of 5 and a page limit; links whose extension is one of the wanted types are collected wherever they are hosted, links without an extension or with a page extension (`.html`, `.php`, ...) are pages to read. It waits the delay (or the host's `Crawl-delay`) between requests to a host, asks robots.txt before each one when that is on, and optionally asks each file's size with `HEAD` (the first 500). Progress goes to a callback; a `CancellationToken` stops it with what it found.
- `feed` parses RSS 2.0, RSS 1.0 and Atom with `roxmltree` (no DTDs), and `FeedFilter` picks an item's enclosures (also Media RSS) and its own link when that is a file, by file type (from the URL, else the announced MIME type) and a case-insensitive regular expression on the title.

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

**Categories.** When a download is added without a folder, the manager picks one from its category (by the extension of the browser's file name hint or the URL) and marks the record `auto_dir`. When it completes, the real file name is known; if it belongs to another category, the file is renamed into that folder. Records keep the category for filtering. The catalog schema is at version 7 (`category` and `auto_dir` from version 2, `queue` and `position` from version 3, `media`, `site`, `speed_limit`, `resume_at` and `site_rule` below).

**Queues.** `Settings::queues` lists them, the main queue (`main`) always first; ids of new queues are made from their names. Every record has a `queue` and a `position`. The scheduler walks the records in position order and starts a queued download when its queue has a free slot, so queues run side by side and each starts its downloads top to bottom. `move_tasks` reorders the selected downloads inside their queues: each queue keeps the positions it holds in the global order, only which download sits in them changes. `set_queue` moves downloads to the end of another queue; removing a queue in the settings moves its downloads to the main queue.

**Confirmation.** A download added with `confirm` (the browser bridge sets it while `confirm_browser_downloads` is on, and the desktop app's Add dialog always sets it for a video page, so its quality is chosen before yt-dlp starts) is created paused with `awaiting_confirmation` in its live state. The Add dialog also sets `from_app`, which the snapshot carries as `from_app` too, so every window can tell where the question belongs: in the main window for what was added there, in the confirmation window for what the browser sent. `resume_all`, `start_queue` and the schedule leave it alone; `confirm` applies the chosen name, folder and queue and either queues it or keeps it paused. The bridge passes the flag on to the extension in its `accepted` and `tasks` answers, so the popup shows such a download as waiting rather than paused (see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md)). The flag is not stored: after a restart such a download is an ordinary paused one.

**Streams.** A record whose URL is an HLS or DASH manifest has `media` (`MediaInfo`: the kind, the chosen tracks and the quality label), stored as JSON in the catalog's `media` column (schema version 4). `add` sets it from `AddOptions::media` or from an `.m3u8` / `.mpd` path, and files such a download under the Video category. `inspect_media` reads the manifest with the download's own headers and returns its qualities for the confirmation dialog; `confirm` stores the user's choice. When the job resolves a choice (or the best quality, when there was none), the manager stores what it resolved to, so a resume after a restart downloads the same tracks. `Settings::ffmpeg_path` (empty: `ffmpeg` on `PATH`) is looked up when a stream starts. Removing or cancelling a stream download deletes its work folder.

**Per-download limits and "Pause until".** A record's `speed_limit` (schema version 6) is that download's own limit; `set_speed_limit` stores it and changes the rate of the bucket the running job shares (a site download hands the lower of the two limits to yt-dlp, which takes it at its next start). `pause_until` pauses a download and stores `resume_at` (Unix seconds, schema version 6); a timer of the manager's own calls `resume_due` every 5 seconds with the wall clock, so a computer that slept through the time, or a Downpipe that was closed then, resumes it as soon as it can. Pausing or resuming by hand, cancelling and completing clear the time.

**Video pages.** `add` recognizes a video page (`downpipe_providers::pages`), stores it under its canonical link, files it as a video and gives the record `site` (`SiteInfo`: the site, the chosen formats in the stream chooser's `MediaSelection` shape, the quality label), stored as JSON in the catalog's `site` column (schema version 5). The scheduler starts such a record with `downpipe_providers::job` instead of the engine, with the tools found from `yt_dlp_path`, `deno_path` and `ffmpeg_path`, the proxy and the speed limit (as yt-dlp's per-download limit). `inspect_site` asks yt-dlp for the page's formats and title for the confirmation dialog, `confirm` stores the choice, and the formats yt-dlp actually took are stored like a stream's resolved tracks. `site_tools` reports the tools with their versions, and `install_site_tools` downloads yt-dlp and Deno into the tools folder when the user asks.

**Download again.** `redownload` resets a completed record and queues it from the same link. With `replace`, the old file is deleted first and the name is kept; otherwise the name is cleared so the engine picks a free one (`name (1).ext`) next to the old file.

**Backup and import.** `downpipe_core::backup` writes the settings and the list (`Manager::backup`) as one JSON file and reads one back as untrusted input: `Backup::parse` checks the structure and each download on its own (`BackupDownload::validated`), `merge_settings` merges queues by id, categories by name and site rules by site, and never takes the paths of the programs Downpipe runs or the remote control, and `Manager::import_downloads` adds what passed, unfinished downloads paused, skipping links already in the list; it writes only to the catalog. Feed subscriptions travel in the same file (`BackupFeed`); the desktop app, which owns `Feeds`, adds them on export and subscribes to them on import with `Feeds::import`, which reads each feed once and remembers its current items without downloading them. `downpipe_core::idm` reads IDM's list for the same import: an exported text or `.ef2` file, or on Windows the numbered keys under `HKCU\Software\DownloadManager`, opened with `KEY_READ` and queried for six named values only. The desktop app's `src-tauri/src/backup.rs` opens the files and keeps what it read until the user picks from the preview, which names entries by position only. See [BACKUP_AND_IMPORT.md](BACKUP_AND_IMPORT.md).

**Proxy and user agent** are part of the settings. Changing either builds a new `Engine` (a new connection pool) with `Engine::derive`, which keeps what the old one learned about hosts that refuse extra connections; running downloads keep the engine they started with.

**Site rules.** `Settings::site_rules` (`rules.rs`) are host patterns (`example.com`, `*.example.com`) with optional overrides. `add` takes a matching rule's folder and queue when none was given; the scheduler looks the rule up again at every start and resume, stores its pattern in the record's `site_rule` (schema version 7) and builds the request with its connections, retries, user agent, referer and a speed limiter shared by the rule's downloads (`DownloadRequest::site_limiter`, next to the download's own `limiter`), on an engine derived for the rule's proxy (cached by proxy, `""` for none). The most specific pattern wins, never a combination. `rules::test_address` answers the settings dialog's "test this address" with the same matching, `rules::problems` returns stable codes for invalid rules (which `update_settings` refuses), and `refused_hosts` lists the hosts the engine remembers as refusing extra connections that no rule holds yet. See [site-rules.md](site-rules.md).

**Schedule.** `downpipe_core::schedule` has the pure rules: `due()` decides whether a start or stop time passed between two timer ticks (so a late tick or a computer that slept through the time still acts), and `DoneWatcher` fires once when the number of running and queued downloads drops to zero after having been above zero.

**Feeds.** `feeds::Feeds` keeps RSS and Atom subscriptions in `feeds.db` next to the catalog (a database of its own, so the catalog's schema is untouched): each feed's address, filter, interval, queue and last check, and every item key it has seen (`item:<guid>` and `url:<link>` for every link an item offers). `add` reads the feed once, refusing pages and invalid XML, and remembers what it holds, or adds the matching items too when asked. `check` adds the filtered links of items none of whose keys were seen, oldest first, with `Manager::add` into the feed's queue, then marks all their keys seen, filtered out or not, so nothing is ever added twice: not after the download is removed, not when the feed renames an item's guid. `run` checks enabled feeds whose interval passed, once a minute. Changes go out on its own broadcast channel. `Manager::http_client` gives the grabber and feeds the download settings' client.

The manager broadcasts `ManagerEvent::{Updated, Removed, Settings}`. The desktop app forwards them to the web view as `downpipe://event`; if the web view falls behind it receives `downpipe://resync` and reloads the list.

Credentials embedded in URLs and custom headers are kept in memory only. That includes the Add dialog's **Request headers** and what **Paste as cURL** (`apps/desktop/src/lib/curl.ts`, a parser for the bash and Windows cmd forms browsers copy) fills in, cookies among them. The database stores the URL with `user:password@` removed.

## Desktop app

`apps/desktop/src-tauri` exposes commands (`list_downloads`, `add_download`, `pause_download`, `remove_download`, `update_settings`, `check_site_rules`, `refused_hosts`, `pick_folder`, `reveal_download`, `inspect_site`, `site_tools`, `install_site_tools`, ...). The app has two windows: `main` (`index.html`, `src/App.tsx`) and `confirm` (`confirm.html`, `src/ConfirmWindow.tsx`). Both web views have only Tauri's `core:default` permission: folder pickers and opening files run in Rust, and file actions take a download id rather than a path, so a web view cannot open arbitrary files; showing, hiding and closing windows happen in Rust too.

The React UI (`apps/desktop/src`) keeps one array of task snapshots and applies events to it (`lib/tasks.ts`). The list draws only the rows in view (`lib/windowing.ts` decides which, from the scroll position and the measured row height), so thousands of downloads cost what a screenful does; moving the keyboard focus goes through the list, which scrolls the row in first; searching, sorting (`lib/sort.ts`), link patterns (`lib/patterns.ts`), the row selection (`lib/selection.ts`), the right-click menu's entries (`lib/menu.ts`) and the keyboard (`lib/shortcuts.ts`) are pure functions with tests. `components/ContextMenu.tsx` draws the menu; the window cancels the web view's own menu outside text fields and the browser's keys (reload, print, find, history, navigation, and the developer tools in release builds).

The browser extension decides what to capture in `apps/browser-extension/src/lib/filters.ts` (file types, excluded sites, minimum size) before anything reaches the app. Alt+click (or Shift+click, `lib/bypass.ts`) on a link lets the browser download it: a content script registered only while capture is on, the key is not off and the extension may access the sites notices the click, and the service worker starts that download as the extension's own, which capture always skips. Its "links on this page" feature injects one self-contained function (`lib/collect.ts`) with `scripting.executeScript` when the user asks, shows the result on an extension page, and sends each chosen link through the same Native Messaging request as a context-menu click; see [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

`src-tauri/src/background.rs` runs beside the window: it forwards events to the web view, shows finish notifications when the window is not in front, keeps the tray tooltip and the taskbar progress current, keeps the computer awake while downloads run (`power::KeepAwake`, on a thread of its own because Windows ties the request to the thread that made it), opens the confirmation window when a browser download waits for confirmation (and brings the main window up for one added there), applies the schedule every 20 seconds (to every queue or the one chosen) and runs the "when done" countdown. `clipboard.rs` watches the clipboard while `watch_clipboard` is on: on Windows it reads the text only when the clipboard's change counter moved, elsewhere it compares the text, and it offers one http(s) link whose file type belongs to a category, or that `downpipe_providers::pages` recognizes as a video page, and that is not in the list yet, compared the way the list keys its links, so a share link of a listed video is skipped too (`downpipe://clipboard`). Every way of adding a link (the Add dialog, pasting, dropping, a clipboard offer) ends in the Add dialog, which adds video pages with `confirm` and `from_app` so the confirmation dialog asks yt-dlp for their qualities in the main window, where the user is; the dialog then has no "Don't ask again", which is about the browser. `tray.rs` builds the tray menu; closing the window hides it when `close_to_tray` is on, and otherwise quits. The main window is built hidden at the end of `setup` (`create: false` in `tauri.conf.json`, since Tauri would otherwise build it before `setup` and the browser bridge would wait for the web view) and shown there, except when the sign-in item or the browser bridge starts Downpipe with `--hidden`.

`confirm.rs` owns the confirmation window, IDM's "Download File Info": downloads the browser sent while `confirm_browser_downloads` is on wait there, one after another, for their name, folder, queue and quality, in `components/ConfirmDialog.tsx` filling a small window with its own taskbar button that comes to the front once per question (on top for a moment, then back in the normal order, with a taskbar flash where Windows keeps the focus elsewhere): one line each for the link, the name (with the size when it is known), the folder, the quality and queue, and the sound track when there is a choice, 520 px wide and as tall as its question needs, neither resizable nor maximizable. The page measures the height (fractional, after every render and once the fonts are in) and calls `fit_confirm_window` with its `devicePixelRatio`; the window is sized in physical pixels from that, rounded up with two spare pixels, so no scrollbar appears at 100, 125 or 150 % display scaling, and only the screen's work area limits it. The main window stays as it was, hidden in the tray or not, and does not ask for them (`lib/tasks.ts` `awaitingIn` splits the waiting downloads by `from_app`). The window is built hidden the first time and kept, hidden, for the next question. The page calls `confirm_window_drawn` once a question is on screen and the window has its height, which shows the window, so it never flashes empty or jumps (a page that does not report within three seconds is shown anyway); the window hides when no browser download waits any more, which the background loop sees in the core's events, so an answer given in either window, or Resume and Remove in the list, ends the question in both. The window's close button asks the page (`downpipe://confirm-close`) to do what Escape does, "Download later" for the download shown. Its Settings links call `open_media_settings`: the main window opens on Settings, Media, and the question is hidden until the main window reports `settings_closed` (or goes to the tray), then shown again with `downpipe://confirm-again` so it is read anew. The pure state machine is unit-tested in `confirm.rs`.

`progress.rs` gives a download started from the confirmation window a progress window of its own, IDM's download dialog: `watch_download` builds `progress-<id>` (the same page, which draws `ProgressWindow.tsx` for such a label) at the confirmation window's place, one step lower and to the right for each window already open, or for a question still waiting. The page follows its download through the core's events and acts on it by id: status, size, speed, time left, resume capability and the segment bar, with Pause or Resume, Cancel and Hide, then Open, Open folder and Close once it is done, or Retry when it failed (`lib/progress.ts` has the texts, with tests). It sizes itself like the confirmation window (`fit_window`), shows once drawn, and names its window after the progress (`set_progress_title`, "45% name"), the only thing the taskbar shows of a download behind other windows. Hiding, closing or cancelling destroys the window rather than keeping it, so many downloads do not keep many web views; the download goes on in the list.

`grab.rs` exposes the site grabber and the feeds: `start_grab` runs `downpipe_crawl::grab` on the async runtime and reports on `downpipe://grab` (`progress` a few times a second, then `done` with the files or `error` with a code), `cancel_grab` stops it; `list_feeds`, `add_feed`, `update_feed`, `remove_feed` and `check_feed` drive `Feeds`, whose changes reach the page as `downpipe://feeds`, and whose loop starts with the app and stops at exit. The header's globe and RSS buttons open `components/GrabberDialog.tsx` (address and filters, progress, then the files with checkboxes, sizes and a queue) and `components/FeedsDialog.tsx`; their pure parts are in `lib/grabber.ts` and `lib/feeds.ts`.

**Translations.** Every visible text lives in `apps/desktop/src/i18n/<language>/` (and `apps/browser-extension/src/i18n/<language>/` for the extension), English in `en/` being the source. A language provides the same structure; the type checker and `catalogs.test.ts` fail when a key, a placeholder or a plural form a language needs is missing. `t()` picks plural forms with `Intl.PluralRules` and fills `{placeholders}`; dates, name sorting and numbers (sizes, speeds, percents, bit rates, with `useGrouping` off so `1023 B` stays one number) use the active language through `Intl`, and decimal fields accept a dot or a comma. Failures are worded by their stable kind (`lib/errors.ts`: the engine's `DownloadError::kind()` and the commands' codes, with `gone` for a download that left the list, so it is not confused with the engine's HTTP 404 `not_found`); the engine's English message stays available as the technical detail. The built-in categories are stored under their English names, which are also their folders' names, and shown translated until renamed (`lib/categories.ts`). The app's language comes from `Settings.language` (empty follows the system); the windows switch it before they render. The tray menu, its tooltip and notifications are translated in Rust (`src-tauri/src/i18n.rs`), with the system language read by `sys-locale`. The extension follows the browser's language.

**Remembered answer.** `Settings::duplicate_answer` is `ask` or the answer chosen with "Don't ask again" (`copy`, `overwrite`, `existing`). The core still refuses or holds the duplicate as above; the desktop app gives the answer instead of asking (`lib/duplicates.ts`), in the main window and in the confirmation window, so both act the same.

**Duplicate links.** `AddOptions.on_duplicate` decides what `Manager::add` does with a link already in the list, finished or not: `refuse` (the default: `CoreError::Duplicate`, so the app can ask), `add_copy` (a new download whose file gets a free name) or `ask` (the browser bridge: the download is held for confirmation with `duplicate_of` set, and the confirmation window asks IDM's question first). A link with a question already waiting is never held twice.

## Browser bridge

`apps/browser-extension` adds **Download with Downpipe** to link context menus and, when the user switches it on, captures downloads the browser starts. Its service worker sends a small, versioned JSON message through Native Messaging. The browser starts `downpipe-native-host` for it, but only because the host manifest lists the extension's id.

With **Media on this page** switched on (the optional `webRequest` permission plus access to `http://*/*` and `https://*/*`), the service worker keeps the `Referer` and `Origin` each media, XHR and fetch request sent (`onSendHeaders`), reads the response headers (`onResponseStarted`) and keeps a list per tab in `storage.session`. `lib/media.ts` decides what counts: manifests by type or extension; files when a media element loaded them or a script fetched them whole, filtered by the media types and minimum size of the options page and by the excluded sites. A content script, registered with `scripting.registerContentScripts` only while the switch is on, puts a Download button over playing videos; it asks the service worker for its own tab's list only. Streams go to the app as a `media` message, single media files as an ordinary `download`. On a video page (YouTube) the popup, the overlay button and a page context menu item send the page itself as a `page` message, for the site provider; that works without detection, from the popup's `activeTab` grant.

`downpipe-native-host` validates each message (`protocol.rs`), forwards it over a local named pipe or Unix socket (`ipc.rs`) and returns the app's answer. The app serves that endpoint from a Tokio task (`server.rs`, started in `apps/desktop/src-tauri/src/main.rs`), validates the request again and calls `Manager::add` with the referrer as an in-memory `Referer` header. When the app is not running, a `download`, `media` or `page` message makes the host start the app next to it (`launch.rs`: a fixed executable name in the host's own folder, with `--hidden`, detached) and wait up to 10 seconds for the pipe before delivering it, unless the message says `start_app: false` (the extension's option). When that fails the host answers `app_not_running`, and captured downloads stay with the browser.

The same crate builds both ends of the pipe, so the end-to-end test (`crates/native-host/tests/e2e.rs`) runs the real host binary against the real server and a real `Manager`, downloading from the test server.

Protocol, install locations and troubleshooting: [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

## Remote control

`crates/remote` (`downpipe-remote`) is an optional HTTP server over the `Manager`, for a phone or another browser. `Settings::remote` (`RemoteSettings`) holds whether it is on, the address and port it listens on (`127.0.0.1:9420` by default) and the SHA-256 of its access token; the token itself is shown once and never stored, and without a hash it cannot be on. `RemoteServer::start` binds an axum router (`api.rs`):

- `GET /api/v1/downloads`, `POST /api/v1/downloads` (`url`, `folder` inside the download folder, `queue`, `paused`, `add_copy`), `POST /api/v1/downloads/{id}/pause` and `/resume`, `DELETE /api/v1/downloads/{id}`, `GET /api/v1/settings`. Rows are `DownloadView`s: name, host, state, sizes, speed, time left, queue and quality, never folders, paths or full links. Errors are `{ "error": code }` with a matching status.
- `GET /` is `remote.html`, `/assets/{name}` and `/icon.svg` its files; nothing else of the bundle is served (`assets::is_servable`).
- A guard in front of everything refuses peers outside local networks when it listens on a network address (`peer.rs`), requests whose `Origin` is not the server, and more than 32 requests at once. The API routes also need `Authorization: Bearer <token>`, checked against the hash (`token.rs`), with a lockout after 10 wrong tokens per peer in 10 minutes (`limiter.rs`). Bodies are capped at 16 KiB.

The desktop app runs it from `src-tauri/src/remote.rs`: it follows the core's `Settings` events, restarts the server when `remote` changes, and serves the page from the app's own bundle through Tauri's asset resolver (from `dist` in development). `set_remote` and `new_remote_token` are the only commands that change `remote`; they make the token and return it once, and `update_settings` keeps the stored `remote` whatever the page sends. The page is a separate Vite entry (`remote.html`, `src/remote.tsx`, `src/RemoteApp.tsx`) with the same design tokens and translations (`i18n/<language>/remote.ts`); `lib/remote.ts` has its API client and helpers, and it has no Tauri code. It takes the token from `#token=` in its address (the settings' link and QR code), keeps it in `localStorage` and reads the list every two seconds while it is visible.

## CLI

`downpipe` talks to the engine directly, without the queue or database. Its resume state is the same `.part.meta` file, so a download started in the CLI can also be resumed by running the command again or with `downpipe resume`.

`downpipe get` refuses a video page (`downpipe_providers::pages`) with a pointer to the desktop app, before any request: downloading it would save the page's HTML, and running yt-dlp without the app's quality chooser would take its best quality, which can be a very large 4K file.

## Test server

`tools/test-server` serves deterministic content (any byte is a function of its offset and a version number) and injects failures through query parameters. The engine and core integration tests run against it on an ephemeral loopback port.

## Planned layers

- **More site providers.** Other sites yt-dlp supports are one entry each in `pages` and `lib/sites.ts`, once their terms and extraction have been checked.
- **Remote control events.** The page polls the list; an event stream would let it follow progress without asking.
