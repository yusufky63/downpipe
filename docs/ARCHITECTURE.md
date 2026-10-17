# Architecture

Downpipe is a set of layers that each know as little as possible about the others. The UI does not know how ranges work, the engine does not know about queues, and nothing below the provider layer (planned) will know what YouTube is.

```text
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

The manager broadcasts `ManagerEvent::{Updated, Removed, Settings}`. The desktop app forwards them to the web view as `downpipe://event`; if the web view falls behind it receives `downpipe://resync` and reloads the list.

Credentials embedded in URLs and custom headers are kept in memory only. The database stores the URL with `user:password@` removed.

## Desktop app

`apps/desktop/src-tauri` exposes commands (`list_downloads`, `add_download`, `pause_download`, `remove_download`, `update_settings`, `pick_folder`, `reveal_download`, ...). The web view has only Tauri's `core:default` permission: folder pickers and opening files run in Rust, and file actions take a download id rather than a path, so the web view cannot open arbitrary files.

The React UI (`apps/desktop/src`) keeps one array of task snapshots and applies events to it (`lib/tasks.ts`).

## CLI

`downpipe` talks to the engine directly, without the queue or database. Its resume state is the same `.part.meta` file, so a download started in the CLI can also be resumed by running the command again or with `downpipe resume`.

## Test server

`tools/test-server` serves deterministic content (any byte is a function of its offset and a version number) and injects failures through query parameters. The engine and core integration tests run against it on an ephemeral loopback port.

## Planned layers

- **Browser extension + Native Messaging host.** The extension observes downloads and media requests and passes candidates to the app over Native Messaging. The host manifest lists the allowed extension ids explicitly; no HTTP port is opened for the extension.
- **Media resolver and providers.** `MediaResolver::resolve(page_url) -> MediaDescriptor` with formats, subtitles and playlists; a `DownloadPlan` of direct HTTP, HLS, DASH and mux steps. yt-dlp runs as an optional external process with argument arrays (never a shell string) and its JSON output is validated before use.
- **Local API.** Loopback-only REST + event stream for a web UI, reusing `Manager`.
