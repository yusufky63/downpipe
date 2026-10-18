# Downpipe

An open-source download manager for Windows, macOS and Linux. Downpipe splits a file across several HTTP connections when that makes it faster, survives pauses, restarts and crashes without starting over, and never publishes a file under its real name until every byte is in place.

It is built as a Rust engine with a desktop app (Tauri), a command-line tool and a browser extension on top. The extension also finds the videos a web page plays, including HLS and DASH streams, and hands them to the app; YouTube pages go to [yt-dlp](https://github.com/yt-dlp/yt-dlp), which Downpipe runs for them. See [Roadmap](#roadmap).

> Status: early development (0.4). The HTTP engine, queue, CLI, desktop app and browser bridge work and are tested. Windows installers and the browser extension are on the [Releases](https://github.com/yusufky63/downpipe/releases) page; they are not code-signed yet, and macOS and Linux builds are built from source for now.

## What works today

| Area | Details |
|---|---|
| Multi-connection HTTP | Byte-range requests over several HTTP/1.1 connections, each with its own TCP congestion window. |
| Adaptive connections | Starts with 2 connections and doubles only while measured throughput improves by at least 10 %, up to 8 per download by default (configurable, max 32). |
| Dynamic segmentation | A connection that finishes takes over half of the largest remaining segment, so a slow segment never holds up the end of a download. |
| Resume | Progress is checkpointed next to the file (`name.part.meta`). Resumes are validated with `If-Range` plus ETag and size checks; if the remote file changed, the download restarts cleanly instead of mixing two versions. |
| Crash safety | Data is synced to disk before the checkpoint that claims it is written. The final file appears only after an atomic rename. |
| Failure handling | Retries with exponential backoff and jitter, honours `Retry-After`, and tells auth, 403, 404 and network errors apart. Hosts that refuse extra connections (403/429/503) get only as many as they accept, remembered per host. Network outages (Wi-Fi drop, network switch, VPN) are waited out for up to 10 minutes, and silent connections are replaced instead of showing as downloading. |
| Integrity | Optional SHA-256 / SHA-512 checksum, verified before the file is renamed into place. |
| Queue | Persistent queue (SQLite) with a concurrency limit, pause/resume/cancel, duplicate detection, and automatic continuation after the app restarts. **Pause until** pauses downloads for 30 minutes, an hour, until 02:00 or a time you choose, and they resume by themselves. Extra queues run next to the main one, each with its own limit; downloads move between queues and up and down inside them. |
| Limits | Global speed limit shared by all downloads, adjustable while they run, and a limit of its own per download (right-click **Speed limit…** or the details dialog) under the global one. |
| Safety | Server-supplied file names are sanitized (path traversal, Windows device names, reserved characters); existing files are never overwritten. |
| Desktop app | Download list with per-segment progress, search and sortable columns, a details dialog, light and dark themes. Multiple selection with checkboxes and a selection bar, a date column, and Downpipe's own right-click menu (open, show in folder, resume, pause, download again, move to queue, reorder, copy link, remove) with keyboard shortcuts. Asks what to do when a link is already in the list, or remembers the answer with **Don't ask again**. Taskbar progress, and the computer stays awake while downloads run. |
| Languages | English, Türkçe, Español, Deutsch, Français, Italiano, Português (Brasil), Русский, 简体中文, 日本語: the app follows the system or the language picked in Settings, the extension follows the browser. |
| Adding links | Paste or drag links onto the window, expand patterns like `file[001-100].zip`, import a `.txt` list, and save the whole list back to a file. Custom request headers, and **Paste as cURL** for a request copied from the browser's developer tools (GET only). Optionally, a copied download link or YouTube link opens the Add dialog by itself. Finished files can be dragged out into other apps. |
| Categories | New downloads go to a folder per file type (Compressed, Documents, Music, Video, Programs, Disk images; editable). If the server's file name shows a different type, the finished file moves to the right folder. Folders you pick yourself are left alone. |
| Expired links | **Use a new link** on a paused or failed download continues it from a fresh link, but only after checking it is the same file (size and ETag); otherwise the partial data is kept and the new link refused. A sign-in or "link expired" page where the file should be stops the download with that reason instead of being saved, and the partial data is kept. |
| After a download | Optional, each off by default: a virus scan with Microsoft Defender (or a scanner command such as `clamscan`) that marks and reports a threat without deleting anything; extracting zip, tar, tar.gz, tar.xz and 7z archives per category or download into a new folder, safe against `../` paths, links and zip bombs; and a command such as `7z x {path}` or `notify-send {name}`, for every download, per queue or per download, run as a program with arguments, never through a shell. |
| Tray and start-up | Closing the window keeps Downpipe in the tray, so the browser can still hand downloads over. Optional start at sign-in, in the tray. A system notification when a download finishes or fails while the window is in the background. |
| Backup and import | **Settings → App → Export settings and list…** saves the settings, queues, categories and the download list to one JSON file, without headers or cookies; **Import…** previews a backup and merges what you tick, skipping links already in the list and adding unfinished downloads paused. IDM's download list comes in the same way, read from IDM's registry keys on Windows or from a list exported from IDM, without ever changing IDM's data. See [Backup and import](docs/BACKUP_AND_IMPORT.md). |
| Schedule | Start and stop times per weekday, and "when all downloads finish": quit, sleep or shut down, after a 60-second countdown you can cancel. |
| Network | Proxy (`http`, `https`, `socks5`) and a custom user agent in the settings; the system proxy is used otherwise. |
| Site rules | **Settings → Sites**: host patterns (`example.com`, `*.example.com`) with their own connections, retries, speed limit, user agent, referer, proxy (or none), folder and queue. The most specific pattern applies, at every start and resume; the details dialog names it. A **Test an address** field shows which rule a link gets before saving, and hosts that refused extra connections are offered as rules. See [site rules](docs/site-rules.md). |
| Capture filters | The extension captures only listed file types (or all of them), never on excluded sites, and leaves files under a minimum size to the browser. One click pauses capture on the current site, and Alt+click (or Shift+click) on a link leaves that one download to the browser. |
| Links on a page | **Download links on this page with Downpipe…** lists a page's links with type filters and sends the ones you tick. The page is read only on that click (`activeTab`), never in the background. |
| Site grabber | IDM's grabber: from one address, Downpipe follows the links of the same site up to a depth (1 to 5) and a page limit, and lists the files of the types you tick (the categories' types plus your own) with their sizes, to add the ones you choose to a queue. It honours `robots.txt` (a switch), waits between requests (500 ms by default, longer when the site's `Crawl-delay` asks), sends no cookies, and can be stopped at any time with what it found so far. |
| Feeds | RSS and Atom subscriptions (podcasts, release feeds): each feed is checked on its interval and the files of new items (enclosures, or the item's link when it is a file) go to the queue you choose, filtered by file type and a regular expression on the title. Every item is remembered, so it is added once and never again, also after you remove its download. A new feed starts from what it holds now unless you ask for that too. |
| File hosts | Google Drive share links download the file: the "can't scan this file for viruses" page of large public files is confirmed for you, and a file that is not shared publicly says so instead of saving a sign-in page. Hosts that need an account, a premium plan or a captcha (Mega and the like) are not scraped. See [File hosts](docs/DOWNLOAD_ENGINE.md#file-hosts). |
| CLI | `downpipe get`, `downpipe resume`, `downpipe probe`. Ctrl+C pauses cleanly. YouTube links are left to the desktop app, where you choose the quality. |
| Media detection | With **Media on this page** switched on (the extension then asks for access to all sites), the extension notes the HLS and DASH manifests and media files a page plays and lists them in its popup, and a **Download** button appears on playing videos, as IDM's download panel does. Media file types, a minimum size and the button can be set under Filters; excluded sites are left alone. The player's own Referer and Origin go with the download; cookies never do. |
| Streams | HLS (master and media playlists, AES-128 segments, transport streams and fragmented MP4, separate audio) and static DASH (separate video and audio). Pick the quality and audio when the download is confirmed. Fragments download in parallel and survive pauses, restarts and crashes. A user-installed FFmpeg merges separate video and audio without re-encoding; without one they are saved as two files. DRM-protected and live streams are refused. |
| YouTube | On a YouTube video page, **Download with Downpipe** (in the popup, on the video's Download button, in the page's context menu, or on a link to a video) sends the page to the app, which runs yt-dlp for it: the confirmation dialog lists the qualities (H.264 first, so the file plays everywhere), the sound tracks and **Audio only**, and FFmpeg joins video and sound into one MP4. Pause and resume work like any download. YouTube links added in the app (typed, pasted, dropped on the window or copied while clipboard watching is on) go through the same quality list before anything starts. yt-dlp and Deno (which yt-dlp needs for YouTube) are found on `PATH`, set in **Settings → Media**, or downloaded there with one button from their official releases, checked by SHA-256; nothing is downloaded without that press. No cookies are sent, so private, members-only and age-restricted videos are not downloaded, and neither are protected or live ones. Following YouTube's terms is up to you; see [Legal](docs/LEGAL.md). |
| Remote control | Optional and off by default: **Settings → Remote** serves a small web page (built for phones, in all ten languages) and an HTTP API to see the list with progress, add links (with a folder inside the download folder and a queue), and pause, resume or remove downloads from a phone or another browser. It listens on this computer only unless you pick the local network, with a warning. Every request needs a random access token, shown once (copy it, or scan the QR code with the address and token) and stored only as a hash; wrong guesses are locked out, other sites cannot call it, and it never shows folders or runs anything. See [Security model](docs/SECURITY_MODEL.md#remote-control). |
| Browser bridge | Chrome, Edge and Firefox extension: **Download with Downpipe** on any link, optional capture of the downloads the browser starts (the browser keeps them if Downpipe is closed), a small window of its own over the browser, like IDM's Download File Info, for the file name, folder and queue (and a stream's or video's quality) before a sent download starts, without opening the main window (can be switched off), and after **Start download** a progress window for it, as in IDM, a notification for every hand-over, and a toolbar popup with the connection status and the live progress of what the browser sent. Talks to the app through Native Messaging and a local pipe, never a network port; the desktop app ships the native host and registers it for your user account when it starts. A download sent while Downpipe is closed starts it in the tray first (can be switched off in the extension). |

What Downpipe does **not** do: it does not bypass DRM, paywalls or logins, it does not read browser cookies (the extension refuses to pass them; only cookies you paste into the Add dialog yourself are sent, and they stay in memory), and it does not send your downloads or links to any server of ours. See [Legal](docs/LEGAL.md).

## Install from source

Requirements: Rust 1.88 or newer, Node.js 22 or newer, pnpm 9, and on Windows the WebView2 runtime (included in Windows 10/11).

Command-line tool:

```powershell
cd C:\path\to\downpipe; cargo build --release -p downpipe-cli
```

The binary is `target\release\downpipe.exe` (or `target/release/downpipe` on macOS/Linux).

Desktop app (development mode):

```powershell
cd C:\path\to\downpipe; pnpm install; pnpm desktop
```

If an installed Downpipe is already running, a second start only brings its window forward. To run a development build next to it, give it its own catalog: `$env:DOWNPIPE_DATA_DIR="C:\path\to\dev-data"; pnpm desktop`.

Desktop installers (the native host for the browser extension is included):

```powershell
cd C:\path\to\downpipe; pnpm install; pnpm --filter @downpipe/desktop tauri build
```

Browser extension:

```powershell
cd C:\path\to\downpipe; pnpm extension
```

Then load `apps/browser-extension/dist/chrome` as an unpacked extension in Chrome or Edge (or `dist/firefox` as a temporary add-on in Firefox). The desktop app registers its browser bridge for your user account every time it starts, so once Downpipe has run, the extension's popup says **Connected to Downpipe**. The Windows uninstallers remove that registration again, unless another copy of the host has taken it over. Step by step, with registering by hand, macOS and Linux paths and the registry keys: [docs/BROWSER_BRIDGE.md](docs/BROWSER_BRIDGE.md).

## Command line

```text
downpipe get <URL> [-o DIR] [-n NAME] [-c MAX_CONNECTIONS] [--limit RATE] [--checksum sha256:HEX] [-H "Name: value"] [--proxy URL] [--stream hls|dash]
downpipe resume <file.part | file.part.meta>
downpipe probe <URL>
```

Examples:

```powershell
downpipe get https://example.org/ubuntu.iso -o D:\ISO --checksum sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
```

```powershell
downpipe get https://example.org/big.zip --limit 4M -c 4
```

Running the same `get` command again after Ctrl+C, a closed terminal or a crash continues the download. `probe` shows the size, whether byte ranges and resume are supported, and the validators the server sends.

An `.m3u8` or `.mpd` link downloads the stream in its best quality (`--stream hls|dash` for links without those extensions). FFmpeg on `PATH` merges separate video and audio.

Exit codes: `0` finished, `1` failed, `2` paused or canceled.

## How the engine works

1. **Open.** The first request is a real `GET` with `Range: bytes=0-`. A `206 Partial Content` answer proves range support and that connection keeps streaming as the first worker, so no request is spent on a separate `HEAD`. A `200` answer means one connection.
2. **Split.** When another connection is allowed, it takes the second half of the segment with the most bytes left. The original connection keeps reading and stops at the new boundary.
3. **Measure.** Every two seconds of saturated transfer the engine compares throughput with the previous connection count and doubles only when it helped.
4. **Checkpoint.** Every five seconds, and always on pause, written data is synced and the finished ranges are recorded in `name.part.meta` with the ETag, Last-Modified and size they belong to.
5. **Finish.** After the last byte, the optional checksum is verified and `name.part` is renamed to `name` (or `name (1)` if that exists by then).

Streams take a different path: the manifest lists fragments, several download at once into a work folder (`name.part/`, one file per finished fragment), and at the end they are joined per track and merged or moved into place.

YouTube pages are downloaded by yt-dlp, run as a separate process with `--ignore-config`, no cookies and Downpipe's own progress lines; pausing ends it and resuming passes `--continue`.

Details: [docs/DOWNLOAD_ENGINE.md](docs/DOWNLOAD_ENGINE.md). Component overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Browser bridge: [docs/BROWSER_BRIDGE.md](docs/BROWSER_BRIDGE.md). Security model: [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md).

## Repository layout

```text
crates/http-engine   downpipe-http   probe, segments, workers, resume meta, checksum, rate limit, HLS/DASH streams
crates/core          downpipe-core   queue, state machine, SQLite catalog, settings
crates/cli           downpipe        command-line interface
crates/native-host   downpipe-native-host   browser bridge: Native Messaging host, protocol, local pipe
crates/providers     downpipe-providers     site providers: YouTube pages through yt-dlp, tool lookup and download
crates/crawl         downpipe-crawl         site grabber (links, robots.txt) and RSS/Atom feed reader
crates/remote        downpipe-remote        optional remote control: token-protected HTTP API and web page for phones
apps/desktop         Tauri 2 + React desktop app (src-tauri is the Rust side)
apps/browser-extension   Manifest V3 extension for Chrome, Edge and Firefox
tools/test-server    fault-injection HTTP server used by tests and for benchmarks
docs/                architecture, engine, security and legal notes
```

## Development

All checks:

```powershell
cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace; pnpm check; pnpm --filter @downpipe/desktop build; pnpm extension
```

Tests run against `tools/test-server`, a local server that can ignore ranges, drop connections, return 429/503 with `Retry-After`, change the file between requests, throttle one region of a file and more. The YouTube provider's tests start a fake yt-dlp and Deno (the test binary itself, copied under their names, see `tools/test-server/src/fake_tools.rs`). Nothing in the test suite touches the internet. To try the app or CLI against it:

```powershell
cargo run -p downpipe-test-server
```

It prints ready-to-use URLs for each failure mode, and `http://127.0.0.1:17655/` lists them as links to right-click in a browser with the extension.

## Roadmap

The order follows one rule: each step has to work end to end before the next one starts.

1. **HTTP engine, resume, adaptive segmentation, queue, CLI, desktop shell** — done (0.1).
2. **Browser bridge.** Chrome/Edge/Firefox extension that hands downloads to the app through Native Messaging (no local HTTP port), with origin checks and no cookies in messages, registered by the desktop app itself — done (0.2). Still to come for the bridge: store-signed extension builds, and a per-site cookie opt-in for downloads that need a signed-in session.
3. **Media detection.** Find the real manifest or media request behind a page instead of `blob:` URLs; HLS and DASH downloads with fragment concurrency, a quality chooser, and a user-installed FFmpeg for merging separate audio and video — done (next release). DRM-protected and live streams are refused.
4. **Site providers.** yt-dlp for sites whose players need site-specific extraction, feeding the same quality chooser: YouTube (watch, Shorts, embedded and `youtu.be` pages), with yt-dlp and Deno found on the machine or downloaded on request and checked by SHA-256 — done (next release). DRM-protected and live videos are refused. Still to come: more sites yt-dlp supports, each after a look at its terms, and playlists.
5. **Local API and web UI.** An optional remote control: a token-protected HTTP API and a web page for phones (list with progress, add links, pause, resume, remove), off by default and on this computer only unless the user picks the local network — done (next release). Then a plugin SDK.

## Legal

Downpipe is intended for downloading files and media that you own, have permission to download, that are made downloadable by the relevant service, or that may otherwise be lawfully downloaded. The project does not provide DRM circumvention and does not guarantee that use with a supported website is permitted by that website's terms.

Downpipe; sahibi olduğunuz, indirme izniniz bulunan, ilgili hizmetin indirilebilir olarak sunduğu ya da başka bir şekilde yasal olarak indirilebilen dosya ve medyaları indirmek için tasarlanmıştır. Proje DRM korumasını aşma imkânı sunmaz ve desteklenen bir sitede kullanımın o sitenin kullanım koşullarınca serbest olduğunu garanti etmez.

More in [docs/LEGAL.md](docs/LEGAL.md).

## License

Apache License 2.0. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
