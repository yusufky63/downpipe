# Changelog

All notable changes to Downpipe. Dates are YYYY-MM-DD.

## Unreleased

The download engine handles what users of other download managers keep running into: hosts that cap connections, silent connections, sign-in pages saved as files, network switches and awkward file names. Five small features users of other download managers asked for come with it.

### New

- **Alt+click lets the browser download a link.** While capture is on, a click on a link with Alt held (Option on a Mac) leaves that one download to the browser instead of Downpipe. The key is set under the extension's **Filters**, **Download in the browser**: Alt, Shift or off. The click is noticed by a small script in the pages, which needs the optional access to the sites you visit (the same access **Media on this page** asks for); the options page offers **Allow access** while it is missing.
- **A speed limit per download.** **Speed limit…** in the right-click menu (for one download or several) and **Speed limit** in the details dialog give a download a limit of its own, under the global one. A running download follows the change at once; a YouTube video takes it at its next start. The limit is stored with the download and shown in its progress window.
- **Request headers and Paste as cURL.** The Add dialog has a **Request headers** field (one `Name: value` per line) and **Paste as cURL**, which reads a command copied from the browser's developer tools (Network, Copy as cURL, in its bash or Windows form) into the link, the headers and, with `-o`, the file name. Cookies are taken only when the pasted command or the field has them. Only GET requests are accepted: a command with `-X POST` or another method, or one that sends data (`-d`, `-F`, `--json`, `-T`), is refused with the reason. Pasting such a command straight into the empty Links field does the same. Headers and cookies stay in memory only, as before, and are never written to disk.
- **Don't ask again for links already in the list.** The question for a link that is already in the list has **Don't ask again, always do this**; the answer is then given without asking, in the main window and for downloads from the browser. **Settings → Downloads → When a link is already in the list** changes it back.
- **Pause until a time.** **Pause until** in the right-click menu pauses the selected downloads for 30 minutes, an hour, until 02:00 tonight, or until a time you choose; they resume by themselves then. The list and the progress window say when. The time is stored with the download, so a download whose time passed while Downpipe was closed resumes when it starts. Pausing or resuming by hand cancels it.

- **Site rules** (**Settings → Sites**). Host patterns, `example.com` for one host or `*.example.com` for a domain and everything under it, each with optional connections, retries, speed limit (shared by the site's downloads), user agent, referer, proxy or no proxy, folder and queue. The most specific matching rule applies, whatever the order: when a download is added (folder, queue) and every time it starts or resumes (everything else), so a rule edited while a download is paused applies to its resume. The details dialog shows which rule a download got. Video pages take the proxy and the speed limit.
- **Test an address** on the Sites tab says which rule a link or host gets, from the same code downloads use, including rules not saved yet and matching rules that are switched off. Patterns that cannot match (a pasted link, a port, a `*` in the middle, `*.com`), duplicates, bad referers and proxies are explained under the field instead of being saved.
- Hosts that refused extra connections this session are listed on the Sites tab with **Make it a rule**.
- **Remote control.** Optional and off by default: **Settings → Remote** serves a small web page for phones and an HTTP API, so another device can see the list with progress, add links (to a folder inside the download folder and a queue) and pause, resume or remove downloads. It listens on this computer only unless you pick the local network, which warns that the token is then the only barrier. Every request needs a random access token, shown once with a copy button and a QR code that carries the address and the token, and stored only as a hash; **New token** replaces it. Wrong guesses lock a device out for a while, other sites' pages cannot call it, peers outside local networks are refused, and it never shows folders or paths, browses files or runs anything. The page and its settings are in all ten languages. API and threats: [Architecture](docs/ARCHITECTURE.md#remote-control), [Security model](docs/SECURITY_MODEL.md#remote-control).
- **Site grabber** (the globe button in the header), as in IDM: enter an address, a depth (1 to 5), a page limit and the file types (tick categories or type your own), and Downpipe reads the site's pages and lists the files it finds, with their sizes from a `HEAD` request, to tick and add to a queue. Only pages of the same site are followed unless you switch that off (files linked from other hosts are listed either way), `robots.txt` is honoured unless you switch that off, there is a delay between requests to a site (500 ms by default, or the site's `Crawl-delay`), and no cookies are sent. **Stop** ends it early with what it found.
- **Feeds** (the RSS button in the header): subscribe to RSS and Atom feeds, choose the file types, a regular expression for titles, how often to check (15 minutes to a day) and the queue, and the files of new items are added by themselves. Each item is added once and never again, also after its download is removed or when the feed changes the item's id. A new subscription starts from the items the feed holds now, unless **Download what the feed holds now** is on. **Check now**, **Edit** and **Remove** per feed; the last check's time or error shows under it.
- **Google Drive.** Share links (`/file/d/<id>/view` and the others) download the file instead of the page, and the "Google Drive can't scan this file for viruses" page of large public files is confirmed automatically. Files that are not shared publicly fail with a message saying so (new error kind `private_file`), Drive's daily download quota and folder links with their own message. Hosts that need an account, a premium plan or a captcha (Mega and the like) are not handled; [DOWNLOAD_ENGINE.md](docs/DOWNLOAD_ENGINE.md#file-hosts) lists what is.
- New crate `downpipe-crawl` (HTML links, `robots.txt`, the grabber, RSS/Atom); `downpipe_core::feeds` with its own `feeds.db`; `Manager::http_client`; engine module `hosts`.

- **After a download** (a new tab in Settings): what happens once a download completes, as JDownloader and IDM users expect. Each step is off until you turn it on, and nothing runs for a download that failed.
  - **Run a command**: a program and its arguments, such as `"C:\Program Files\7-Zip\7z.exe" x {path} -o{folder}`, with `{path}`, `{name}`, `{folder}` and `{url}` filled in. One for every download, one per queue (in **Queues**) and one per download (in the Add and confirmation dialogs and the details), the download's own first. It runs as a program with an argument list, never through a shell, so file names and links cannot add commands; it is stopped after a time limit (10 minutes by default), and what it printed shows in the details and the log.
  - **Scan downloads for viruses**: with Microsoft Defender on Windows (`MpCmdRun.exe -Scan -ScanType 3 -File ... -DisableRemediation`), or a scanner command such as ClamAV's `clamscan` elsewhere. The result shows on the download: nothing found, a threat (a red **Threat** badge, a notification, and nothing extracted or run), or no scanner. Nothing is deleted or quarantined.
  - **Extract archives**: zip, tar, tar.gz, tar.xz and 7z, into a new folder next to the archive, switched on per category (**Categories → Extract archives**) or per download. Entries that would land outside that folder (`../`, absolute and drive paths), links and archives that would expand far beyond their size (zip bombs) are refused; the details list what was left out and open the folder. Optionally the archive is deleted once everything in it was extracted. RAR is not supported; 7-Zip as the command can extract it.
- Core: `downpipe_core::post` (`PostActions`, `PostSettings`, `PostReport`), `Manager::set_post_actions`, `AddOptions::post`, `ConfirmOptions::post`, the snapshot's `post_actions`, `post` and `post_step`, `Settings::post`, `Queue::command`, `Category::extract`; catalog schema version 8.
- **Downpipe starts from the browser.** A link, stream or video page sent from the browser while Downpipe is closed starts it in the tray and is then added, instead of failing with "Downpipe is not running" (IDM does the same). The bridge starts only the Downpipe next to it, never a program a message names, waits at most 10 seconds and otherwise gives the old answer, so a captured download stays with the browser. **Start Downpipe when it is closed** on the extension's Filters page turns it off. Bridge: an optional `start_app` field on `download`, `media` and `page`.
- **Backup and restore.** **Settings → App → Export settings and list…** saves the settings (queues, categories and site rules included), the download list (links, folders, names, states and progress) and the feed subscriptions to one JSON file, never with headers or cookies. **Import…** shows what a backup holds and takes the settings, the downloads, the feeds or any of them: links already in the list are skipped, unfinished downloads are added paused, no file is replaced, and every entry is checked on its own, so a damaged one is left out instead of failing the import. Imported feeds remember what they hold now instead of downloading it. The paths to FFmpeg, yt-dlp and Deno and the remote control are never taken from a file.
- **Import from IDM.** **Settings → App → Import from Internet Download Manager** reads IDM's download list, on Windows straight from IDM's registry keys (link, file name and size only, opened read only; files go to Downpipe's category folders, since IDM keeps its save folder in no readable value) or from a list exported from IDM as text or `.ef2` (links and file names only; cookies and headers in the file are skipped), shows it with checkboxes and adds the ticked downloads paused. IDM's data is never changed. What is read is listed in [docs/BACKUP_AND_IMPORT.md](docs/BACKUP_AND_IMPORT.md).

### Fixed

- **Hosts that cap connections.** A server that refuses extra connections with `403`, `429` or `503` while the first one downloads no longer fails the download: Downpipe continues with the connections it accepts and remembers that number for the host until it quits, so the next download from it starts there.
- **Network drops.** Downloads waiting to retry after the network dropped or changed (Wi-Fi, a switch to another network, a VPN reconnect) continue by themselves once requests get through again. Network failures are retried for up to 10 minutes without a byte arriving instead of a fixed number of attempts, so a short outage no longer fails a download. A resume started while offline waits too.
- **Silent connections.** A download never shows as downloading while nothing arrives: after 10 seconds without data it shows **Retrying** with the reason (no data from the server, network unavailable, the server asked to slow down, server error), in the list, the details and the progress window. A server that accepts a connection but never answers is asked again after 60 seconds instead of holding the download forever.
- **Web pages instead of files.** When a new download, a resume or a new link gets a web page (a sign-in or "link expired" page) where a file is expected, the download stops with **The link has expired** or **The server asks you to sign in** instead of saving the page or throwing away the downloaded part. The partial data is kept, and the list offers **Use a new link…** in place of **Retry** (also for `403` failures).
- A segment that kept failing could retry forever, because each retry reset its failure count; only progress resets it now.
- **File names.** Raw UTF-8 names in `Content-Disposition` (which servers send although the standard does not allow it) are read instead of falling back to the URL; percent-encoded and RFC 2047 (`=?UTF-8?B?...?=`) names, `filename*0*=` continuations, a missing disposition type and an unterminated quote are understood. After a redirect to a storage URL named by a hash, the link's own name (`setup.exe`) is kept. Invisible direction marks that disguise extensions are removed, `COM¹`-style device names are made safe, and a name cut to length no longer ends in a dot. A single-connection download that fails no longer leaves its partial file behind, which pushed the retry to `name (1)`.
- `downpipe get` says why it is waiting (`network down, retrying`, `no data, reconnecting`).

### Engine

- `DownloadRequest::max_retries` and `DownloadRequest::site_limiter` (a site's speed limit, next to the download's own), `EngineConfig::no_proxy`, `Engine::derive` (another proxy or user agent, sharing the limiter and host memory, also used when the settings change, so a proxy change no longer forgets which hosts refused connections) and `Engine::connection_limits`.
- Catalog schema 7: `site_rule` on each download. yt-dlp gets `--proxy ""` for a rule without a proxy.
- New error kinds `expired_link` and `login_required`; `Progress::waiting` and the snapshot's `waiting` give the reason while a download waits; `EngineConfig::offline_grace` (10 min) and `response_timeout` (60 s); `Engine::connection_limit` reports a host's remembered limit.
- Test server: `max_conns` and `limit_status` (refuse connections beyond N), `stall_at`/`stall_times` and `hang_times` (silence mid-body or before the headers), `html`/`html_after` and `/login` (web pages instead of the file), `cd` (a raw `Content-Disposition`), `fail_from`, and `TestServer::stop`/`restart` to take the server off the network and back.

### App and extension

- **Failures in your language.** Every reason the engine gives for a failed download (file not found, the server refused it, network failure, disk full, DRM, a live stream, yt-dlp could not read the page and so on) is shown translated in the list, the details, the progress window, the confirmation dialog and messages. The engine's own English text stays under **Technical details** in the details dialog (and the confirmation dialog), for searching or reporting. The popup words the same reasons, with the app's text in the row's tooltip, and words the bridge's remaining codes too.
- **Numbers in your language.** Sizes, speeds, percents and bit rates follow the language: `1,5 MiB` and `%50` in Turkish, `50 %` in German and French. English is unchanged.
- **Decimal commas.** The speed limit in Settings and the minimum sizes in the extension's Filters accept `1,5` as well as `1.5`, and show stored values with the language's decimal sign. An invalid speed limit is pointed out instead of quietly meaning "no limit".
- **Category names in your language.** The built-in categories (Compressed, Documents, Music, Video, Programs, Disk images) show translated until you rename them; their folders on disk keep their names. Searching finds a category by either name.
- **A smaller Download button over videos.** The button is now a small icon with a menu arrow, and you can drag it anywhere inside the video; the place is remembered per site. Its menu has **Hide on this site**, which adds the site to a list of its own under Filters → Media on pages (detection and capture still work there), and Filters has **Show it only while the pointer is over the video**.
- **No double downloads.** A capture the app accepts after the browser stopped waiting for it (the app was slow to answer) now removes the browser's copy too, in Chrome, Edge and Firefox, unless the browser already finished it. In Firefox, a download that finished before it could be paused is no longer handed to the app a second time. Each browser download is offered once, also when the extension's service worker restarts in the middle.
- **Large lists stay fast.** The download list draws only the rows in view, so thousands of downloads scroll and update like a few. Keyboard selection, checkboxes, the right-click menu and dragging files out work as before.
- Bridge: `tasks` answers carry `error_kind` next to `error` for failed downloads.

## 0.4.0 — 2026-10-18

Media detection: the videos and audio a web page plays, including HLS and DASH streams behind `blob:` players, download in Downpipe. YouTube pages download through yt-dlp.

### New

- **YouTube.** On a YouTube video page (watch, Shorts, embedded players, `youtu.be` links), **Download with Downpipe** sends the page to the app: in the popup's **Media on this page** (also with detection off), on the Download button over the video, as **Download this video with Downpipe** in the page's context menu, and on links to video pages. The app hands the page to [yt-dlp](https://github.com/yt-dlp/yt-dlp), because YouTube's player no longer fetches files a download manager can take over.
- The confirmation dialog lists what yt-dlp finds: one quality per resolution and frame rate (H.264 before AV1 before VP9, so the result plays everywhere), the sound tracks per language and codec, and **Audio only**. The file is named after the video's title. Without FFmpeg, qualities that need video and sound joined cannot be started, and the dialog says why; **Audio only** and qualities with their own sound still work.
- yt-dlp's progress shows in the list like any download, with the site and quality (`YouTube · 1080p`). Pausing ends yt-dlp and everything it started; resuming continues its partial files. Cancelling deletes them. FFmpeg joins video and sound into one MP4 (or MKV when the formats need it).
- YouTube links added in the app become site downloads too, under one link per video: the same video from a share link, a playlist or with a timestamp is recognized as a duplicate. Whether typed or pasted into the Add dialog, dropped on the window or copied to the clipboard, they go to the same quality list (**Next** in the Add dialog) instead of starting at once in yt-dlp's best quality, which can be a very large 4K file. With several links, each video asks in turn.
- **Offer to download copied links** also offers copied YouTube video links, unless the video is already in the list.
- `downpipe get` with a YouTube link stops with a pointer to the desktop app instead of saving the page's HTML.
- **Settings → Media → Videos on pages:** paths for yt-dlp and Deno (empty: Downpipe's tools folder, then `PATH`; Node 22 or newer stands in for a missing Deno), the status of yt-dlp, the JavaScript runtime and FFmpeg with their versions, and a **Download yt-dlp and Deno** button that fetches the official builds from GitHub and checks each against the SHA-256 its release publishes. Nothing is downloaded until it is pressed; pressing it again updates yt-dlp.
- Bridge: a `page` message (`page_url`, `title`) for video pages, accepted only for pages the app's provider takes (otherwise `unsupported_page`), and `page` in `pong`'s `features`.

- **Media on this page** in the extension's popup (a switch, off by default): the extension notes the HLS (`.m3u8`) and DASH (`.mpd`) manifests and the media files a page plays, per tab, and lists them with a **Download** button each. Turning it on asks the browser for the `webRequest` permission and access to all sites; turning it off removes both and forgets the lists.
- A stream is listed once, by its master playlist, instead of once per quality and audio playlist with random file names (on X, a broadcast replay came up as five rows for two videos). The extension reads each new HLS playlist once itself, without cookies, at most 256 KiB and 5 seconds, and hides the playlists a master lists, wherever they are, also when the player fetched them first. Rows are named after the page (`Home / X`, `Home / X (2)` for a feed) and say how many qualities a stream has (`HLS · 3 qualities`); the address stays in the tooltip.
- A **Download** button over playing videos (big enough, not DRM-protected) while detection is on. With one stream found it sends it; with several it lists them. Pages cannot click it for you.
- **Media on pages** in the extension's options page (now called **Filters**), like IDM's download panel settings: the Download button on or off, the media file types that are listed, and a minimum size (256 KiB by default). The excluded sites list applies to media detection too.
- Detected media keeps the `Referer` and `Origin` the player's own request sent, and Downpipe sends them with the manifest and every fragment. Files that a script fetches in pieces by query string (`bytestart`, `byteend`, `range`) are listed as the whole file.
- HLS downloads: master and media playlists, the best quality or the one you pick, AES-128 encrypted segments, transport streams and fragmented MP4 with an init section, byte-range segments, separate audio renditions.
- DASH downloads: static manifests with segment templates (numbers or a timeline), segment lists or whole files per representation, separate video and audio.
- Fragments download several at a time with retries; each finished fragment is kept as its own file, so pausing, closing Downpipe or a crash costs at most the fragments in flight. A resume checks that the stream still lists the same fragments.
- The confirmation dialog of a stream sent from the browser has a **Quality** list (resolution, bit rate and, for DASH, the estimated size) and an **Audio** list when there is a choice. Only the manifest is read for it; no media is fetched before **Start download**.
- FFmpeg, when installed (on `PATH`, or set in **Settings → Media**), merges separate video and audio into one file and turns transport streams into MP4, without re-encoding. Downpipe does not ship FFmpeg; without it the tracks are saved as `name.mp4` and `name.audio.m4a`, and the dialog says so beforehand.
- Stream downloads in the list show the kind and quality (`HLS · 720p`), an approximate size until the last fragment is in (`~8.1 MiB`), and fragment progress in the details.
- `.m3u8` and `.mpd` links added by hand (Add dialog, `downpipe get`) download as streams. `downpipe get --stream hls|dash` forces it for links without those extensions.
- Bridge: a `media` message (`manifest_url`, `kind`, `page_url`, `referrer`, `origin`, `title`) with the same validation as `download`, and `features: ["media"]` in `pong` when both the bridge and the app can take streams.
- Test server: `/hls/` and `/dash/` streams with checkable fragments and options for encryption, separate audio, live, DRM and failing fragments; `flat=1` puts the quality and audio playlists beside the master (`/hls/v1.m3u8`); `--files DIR` serves a folder, for example FFmpeg output and a page that plays it.

- **Download progress windows**, as in IDM: **Start download** in the confirmation window turns the question into a small window that follows the download, with its status, size, speed, time left, whether it can resume and the segment bar, and **Pause** / **Resume**, **Cancel** and **Hide**. Hiding only closes the window; the download goes on. When it finishes the window offers **Open**, **Open folder** and **Close**, and **Retry** when it failed. Several downloads get a window each, one step apart, and the taskbar shows each one's progress in its title. **Download later** still just closes the question.
- **Ten languages:** English, Türkçe, Español, Deutsch, Français, Italiano, Português (Brasil), Русский, 简体中文 and 日本語, for the window, the confirmation and progress windows, the tray menu, notifications and the browser extension. **Settings → App → Language** picks one; by default Downpipe follows the system, and the extension follows the browser's language. Dates, plurals and name sorting follow the language too.
- **Selecting several downloads** is visible now: a checkbox on every row and one in the header for everything shown (Shift selects a range). With two or more selected, a bar over the list offers **Resume**, **Pause**, **Remove…**, **More** (the right-click menu) and clearing the selection.
- **Date** column: when a download finished, or when it was last tried. Today shows the time, this year the day and month. The tooltip has both the date added and finished, and the column sorts.
- **Already in the list?** Adding a link that is already in the list, finished or not, asks as IDM does: add it again with a numbered file name, add it again and replace the existing file, or show the existing download if it is finished (its window with **Open** and **Open folder**) and otherwise resume it. Several such links get one answer. A link sent again from the browser asks the same question in the confirmation window before its usual question, and is never asked about twice at the same time.

### Changed

- Downloads sent from the browser wait for their answer in a small window of their own over the browser, like IDM's Download File Info, instead of bringing up the whole app: the main window stays in the tray if it was there. The window asks for one download after another, comes to the front once (its taskbar button flashes where Windows keeps the focus elsewhere), then keeps the normal window order, so a click on another window puts it behind; it has its own taskbar button and hides once nothing waits. It is compact: one line each for the link, the name (with the size when it is known), the folder, the quality (streams and video pages) next to the queue, and the sound track when there is a choice, without explanations. It is as tall as the question needs, so nothing scrolls, whatever the display scaling. The confirmation in the main window got the same compact layout. Closing it or pressing Escape is **Download later** for the download shown. Its Settings links open the main window on **Settings → Media** and bring the question back when Settings close. Video pages added in the app itself are still asked in the main window.
- The popup shows **Connected** in its header instead of a status box while the app is reachable, so the media and download lists fit in the browser's 600 px popup.
- Catalog schema version 5 (a `media` column, and a `site` column for video pages); older catalogs are upgraded on start.
- Quality lists name the codecs (`H.264 + AAC`), and audio tracks their format.
- Media detection no longer lists the requests YouTube's player makes to its video servers; the page itself is offered instead.
- The Download button over a video on a page with several videos (an X timeline, a post with a quoted video) offers that video's stream only, matched by the id its poster image shares with its playlist, instead of every stream on the page.
- A tab title of one or two characters (X shows "X" until a post has loaded) no longer names downloads, so videos from X are not all saved as `X.mp4`.
- A link already in the list is no longer added silently when the first download has finished; the question above is asked instead. The bridge answers such a link from the browser with `accepted` and `awaiting_confirmation`, and a third copy while the question waits with `duplicate`.

### Fixed

- Page titles with a slash, such as X's "SpaceX (@SpaceX) / X", named the download after the part behind the last slash (`X.mp4`); slashes now become dashes.

### Not downloaded, on purpose

- DRM-protected streams (HLS `SAMPLE-AES` or any key format other than plain AES-128, DASH `ContentProtection`) are refused before any media is requested, and the overlay button does not appear on videos that use Encrypted Media Extensions.
- Live streams (HLS without `#EXT-X-ENDLIST`, DASH `type="dynamic"`).
- YouTube videos yt-dlp reports as DRM-protected, and live or upcoming ones. No cookies are sent, so private, members-only and age-restricted videos fail with YouTube's reason.

## 0.3.0 — 2026-10-17

The first published release: the IDM features that save the most time day to day, plus everything listed under 0.2.0 and 0.1.0, which were never released on their own.

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

## 0.2.0 — not released on its own

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

## 0.1.0 — not released on its own

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
