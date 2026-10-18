# Browser bridge

The browser bridge hands links and downloads from Chrome, Edge and Firefox to the Downpipe desktop app. No network port is involved at any step:

```text
extension (service worker)
   |  Native Messaging: the browser starts downpipe-native-host and talks
   |  to it over stdin/stdout, only for extension ids the host manifest lists
   v
downpipe-native-host
   |  Windows: named pipe \\.\pipe\downpipe-bridge-<user>
   |  macOS / Linux: Unix socket in a folder only the user can open
   v
Downpipe desktop app  ->  Manager::add  ->  download queue
```

| Part | Where |
|---|---|
| Extension (Manifest V3, TypeScript) | `apps/browser-extension` |
| Native host, protocol, pipe client and server | `crates/native-host` (`downpipe-native-host` binary and library) |
| Pipe server inside the app | `apps/desktop/src-tauri/src/main.rs` (`start_browser_bridge`) |
| Registration at app start | `apps/desktop/src-tauri/src/main.rs` (`register_browser_host`), `crates/native-host/src/manifest.rs` (`sync`) |
| Host bundled with the app | `apps/desktop/scripts/sidecar.mjs`, `bundle.externalBin` in `apps/desktop/src-tauri/tauri.conf.json` |

## Registration by the app

The desktop installers ship `downpipe-native-host` next to the app's own executable: in the install folder on Windows (`%LOCALAPPDATA%\Downpipe` for the default per-user install), in `Downpipe.app/Contents/MacOS` on macOS and in `/usr/bin` for the Linux packages. Every time Downpipe starts, it checks Chrome, Edge, Chromium and Firefox and writes the registration again for a browser whose registration

- is missing,
- points at a manifest that cannot be read or is not a Downpipe host manifest,
- starts another copy of the host (an older install, a build from source), or
- does not allow the Downpipe extension.

A registration that already starts this copy for the Downpipe extension is left untouched. Extension ids someone added to the manifest (a store build, an unpacked copy with another key) are kept when it is rewritten. The app only writes for the current user, exactly as `downpipe-native-host install` does: registry keys under `HKEY_CURRENT_USER` and manifest files in the user's folders. It never writes `HKEY_LOCAL_MACHINE` or system folders, and needs no administrator rights. On macOS and Linux only browsers with a profile folder are registered; a browser installed later is picked up at the next start.

So with an installer the steps are: install Downpipe, start it once, load the extension. Browsers read the registration each time the extension calls the host, so a browser that was already open does not need a restart.

**Settings → Browser bridge** in the app lists the state for each browser and has **Register now**. Turn off **Register with browsers at start-up** to manage the registration yourself, for example to keep the browsers on a host you built from source; Downpipe then leaves it alone. The setting is stored with the download catalog.

The AppImage is the exception to "next to the app": its files live in a mount that changes at every start. The registration then follows the running copy, which works while Downpipe runs; while it is closed the popup reports the bridge as missing rather than Downpipe as not running.

### Uninstalling

The Windows uninstallers remove the registration again, for the user who runs the uninstaller, with the host they are about to delete: `downpipe-native-host uninstall --only-if-host <that host>`. Only a browser whose registration still starts that host is unregistered. One that a build from source or another Downpipe install has registered since keeps its registration, and so does one whose manifest cannot be read. A failure never stops the uninstall; the worst case is a registration left behind, which browsers report as a missing bridge.

- **NSIS installer (`-setup.exe`):** `apps/desktop/src-tauri/installer-hooks.nsh` copies the host to the uninstaller's temporary folder before the files are deleted and runs that copy once the uninstall has gone through. An uninstall that stops early (Downpipe was running and you chose not to close it) therefore leaves the registration alone. It also removes the empty manifest folders in `%LOCALAPPDATA%\Downpipe`, which for the default per-user install is the install folder itself.
- **MSI:** `apps/desktop/src-tauri/browser-bridge.wxs` runs the installed host just before Windows Installer removes the files. The MSI installs for the whole machine, but each user who started Downpipe has a registration of their own, and only the uninstalling user's is removed. Other users keep theirs, pointing at a host that is gone, until they delete the registry keys and manifests listed under [Registering by hand](#registering-by-hand).

Most upgrades keep the registration, since the new version puts the host back at the same path. A silent NSIS install over an existing one (`/S`) does not run the old uninstaller, an NSIS uninstaller started with `/UPDATE` (Tauri's flag for updates) skips the removal, and so does the old MSI during a major upgrade. The exception is installing a newer NSIS version by hand and keeping the installer's default choice, **Uninstall before installing**: the old version's uninstaller removes the registration, and the new version writes it again when it starts. The installer's last page starts Downpipe unless you clear its run option, so the browsers report a missing bridge only until then. That gap is accepted rather than worked around: an installer that registered the host itself would override people who turned the start-up registration off.

macOS (moving `Downpipe.app` to the Trash) and the Linux packages have no uninstall step that could do this. Run `downpipe-native-host uninstall` from the install folder before removing Downpipe, or delete the manifest files listed under [Registering by hand](#registering-by-hand).

## Set up from source

1. Build the extension and start the app:

   ```powershell
   cd C:\path\to\downpipe; pnpm install; pnpm extension; pnpm desktop
   ```

   `pnpm desktop` builds `downpipe-native-host` first (`pnpm --filter @downpipe/desktop sidecar`), and the app registers `target\debug\downpipe-native-host.exe` for your user account when it starts, as described above.

2. Load the extension:
   - **Chrome / Edge:** open `chrome://extensions` (or `edge://extensions`), turn on Developer mode, choose **Load unpacked** and select `apps/browser-extension/dist/chrome`. The public key in the manifest pins the extension id to `efgicfboaldflckpncfdojkeoaamflpf`, which is the id the host allows by default.
   - **Firefox 128 or newer:** open `about:debugging#/runtime/this-firefox`, choose **Load Temporary Add-on** and select `apps/browser-extension/dist/firefox/manifest.json`. Temporary add-ons are removed when Firefox closes; a permanent install needs a signed build.

3. Open the Downpipe toolbar popup. It should say **Connected to Downpipe**.

4. Try it against the local test server:

   ```powershell
   cargo run -p downpipe-test-server
   ```

   Open `http://127.0.0.1:17655/`, right-click a link and choose **Download with Downpipe**. The download appears in the app straight away. Right-click an empty part of the page and choose **Download links on this page with Downpipe…** to try the links page with every sample at once. The sample addresses have no file extension, so they count as web pages: the list shows them all, with none selected.

   Media detection needs a page that really plays something. Make a stream with FFmpeg, add a page that plays it (for example with hls.js), and serve the folder with `--files`:

   ```powershell
   cargo run -p downpipe-test-server -- --files C:\path\to\media
   ```

   The folder is then under `http://127.0.0.1:17655/files/`. The server's own `/hls/master.m3u8` and `/dash/manifest.mpd` streams have checkable but unplayable fragments; they are for tests and for adding by hand in the app.

To use the bridge without the app registering it (the setting is off, or you only built the host), register by hand. No administrator rights are needed:

```powershell
cd C:\path\to\downpipe; cargo build --release -p downpipe-native-host; .\target\release\downpipe-native-host.exe install
```

`install` writes one manifest per browser and, on Windows, points the browser's registry key at it (see [Registering by hand](#registering-by-hand)). `downpipe-native-host status` shows what is registered and `downpipe-native-host uninstall` removes it; with `--only-if-host <path>` it removes only registrations that start the host at that path. If you move the executable, run `install` again: the manifest stores its absolute path. While the app's start-up registration is on, the next start of the app points the browsers back at the host next to the app.

## Building the installers

```powershell
cd C:\path\to\downpipe; pnpm install; pnpm --filter @downpipe/desktop tauri build
```

Before it compiles the app, `tauri build` runs `pnpm sidecar`, which builds `downpipe-native-host` in release mode and copies it to `apps/desktop/src-tauri/binaries/downpipe-native-host-<target triple>` (`.exe` on Windows), the name Tauri's `bundle.externalBin` looks for. With `--target`, the script builds the host for that target too. The installers then put the host next to the app under its plain name, `downpipe-native-host`.

`apps/desktop/src-tauri/build.rs` stops tauri-build from also copying that file into `target/<profile>/`, where Cargo already builds the workspace's own host; the bundler reads `binaries/` directly. That keeps `cargo check --workspace` working before `pnpm sidecar` has ever run.

## Using it

**Download with Downpipe** appears in the context menu of http and https links. The link goes to the app with the page it was on and a referrer. The referrer follows the browser's default policy (`strict-origin-when-cross-origin`): the full page address for links on the same site, only the site's origin for other sites, and nothing from an https page to an http link.

**Downpipe closed?** A download, stream or video page sent while the app is closed starts it first, in the tray, as IDM does: the host starts the app next to it with `--hidden`, waits up to 10 seconds for its pipe and then hands the download over (see [Starting the app](#starting-the-app)). **Start Downpipe when it is closed**, at the end of the Filters page, turns this off; the extension then reports that Downpipe is not running, as before. Checking the bridge (opening the popup) and live progress never start the app; while the option is on, the popup says that a download you send starts Downpipe instead of asking you to open it.

**Capture browser downloads** (off by default) is a switch in the popup. Turning it on asks the browser for the `downloads` permission; turning it off removes the permission again. While it is on:

- A download the browser starts is offered to Downpipe. If the app accepts it (or already has it), the browser's copy is cancelled and removed from the browser's download list.
- If Downpipe is not running (and cannot be started, or starting it is turned off) or refuses it, the browser keeps downloading as if the extension were not there, and the popup notes that the browser kept the file.
- The browser waits 5 seconds for the app's answer. When the app accepts later (the host itself waits up to 10 seconds for it, plus up to 10 seconds more when it starts Downpipe first), the browser's copy, which went on meanwhile, is cancelled and removed then, unless the browser already finished it; a finished file is never erased from the browser's list, which would hide it beside the app's copy. Each browser download is offered once: the ids already offered are kept in session storage, so a second event for the same download (after the service worker restarted, for one) does not send it again. Nothing is kept for later, so neither a browser nor an app restart sends a capture again; downloads the browser brings back after its restart are interrupted ones, which capture skips.
- Always skipped: downloads in private windows (the app keeps a history of its downloads), `blob:` and `data:` URLs (they only exist inside the browser), and downloads the extension started itself.
- Then the capture filters below decide. A download they leave out is never touched: the browser downloads it as usual, with no notification.
- Chrome and Edge decide in `downloads.onDeterminingFilename`, after the server's file name, type and usually its size are known. Firefox has no such event; there the filters are checked in `downloads.onCreated`, and a download they let through is paused, offered to Downpipe and resumed if Downpipe does not take it. A download that finished before the pause (a small file on a fast line) stays with the browser. The hand-over is `apps/browser-extension/src/lib/handoff.ts`, tested against fake `downloads` and host answers in `handoff.test.ts`.

**Download in the browser.** With capture on, a click on a link with Alt held (Option on a Mac) leaves that download to the browser, once. Under **Filters**, **Download in the browser** sets the key: Alt (the default; Chrome and Edge save a link with Alt+click anyway), Shift, or off. A small content script notices the click; the service worker registers it with `scripting.registerContentScripts` only while capture is on, the key is not off and the extension has access to `http://*/*` and `https://*/*` (the optional access **Media on this page** also asks for; the options page offers **Allow access** while it is missing). The script reads the key from the settings and does nothing else until such a click: a real primary-button click (`isTrusted`) with only that key, on an `a[href]` or `area[href]` with an http or https address. It cancels the click (and with it the browser's own Alt+click download) and sends that address to the service worker, which checks the settings again and calls `downloads.download`. Downloads the extension starts are always skipped by capture, so exactly this one stays in the browser. Ctrl and Cmd clicks stay the browser's.

Downpipe never receives browser cookies. Downloads that only work while you are signed in to a site, and downloads that come from submitting a form, fail in Downpipe. Pause capture for those sites, or let the browser download them; a per-site cookie opt-in is on the roadmap.

**Filters** opens from the link under the capture switch in the popup (it is also the extension's options page). Its first three parts are the capture filters; the fourth, **Media on pages**, is for media detection (below), the fifth, **Download in the browser**, sets the key for letting the browser download a clicked link itself, and the sixth, **Desktop app**, holds **Start Downpipe when it is closed** (on by default). It works like IDM's file type settings. Nothing changes until **Save** is pressed; invalid entries are pointed out under their field and nothing is saved until they are fixed.

- **File types.** Only downloads whose file name ends in one of the listed extensions are captured. The list is separated by spaces or commas; `.zip`, `*.zip` and `ZIP` all mean `zip`, and compound extensions such as `tar.gz` work. The name is the browser's final file name (in Chrome and Edge it already includes the server's `Content-Disposition` name); when the browser has none, the last segment of the URL path, so `get.php?file=a.zip` counts as `php`. The default list covers archives (`zip`, `7z`, `rar`, `tar`, `gz`, `xz` and more), programs and packages (`exe`, `msi`, `apk`, `dmg`, `pkg`, `deb`, `rpm`, `appimage`), disk images (`iso`, `img`, `bin`, `vhd`), documents (`pdf`, `epub`, Office and OpenDocument files), audio (`mp3`, `m4a`, `flac`, `ogg`, `wav` and more) and video (`mp4`, `mkv`, `webm`, `mov`, `avi` and more). Web pages, images and scripts stay in the browser. **Capture all file types** ignores the list (and keeps it for later); **Restore the default list** brings the defaults back.
- **Excluded sites.** One host per line. They apply to media detection too. `example.com` matches only that host; `*.example.com` matches `example.com` and all of its subdomains. A download is left to the browser when its URL, the address it was redirected to, or the page it started on (the referrer) is on one of these sites. Matching ignores case, ports and paths, and pasted URLs are reduced to their host. International names are stored in their ASCII (punycode) form, which is how URLs carry them, so `bücher.de` is saved as `xn--bcher-kva.de`.
- **Minimum size.** Downloads smaller than this, in KiB or MiB (decimals with a dot or a comma, `1.5` or `1,5`), stay in the browser. 0 turns it off. When the server does not send a size, the browser reports it as unknown, and such downloads are always offered to Downpipe.

The filters are stored with the other extension settings in `storage.local`. Settings saved by an earlier version get the defaults for the new fields.

**Pause capture on this site** appears under the capture switch while capture is on and the current tab is an http or https page. It adds the tab's host to the excluded sites; **Resume capture on this site** removes that entry again. A wildcard entry that already covers the site is shown instead of the button, since removing it would affect other sites; edit it under Filters. The popup reads the tab's address through `activeTab`, which the browser grants when the popup is opened.

**Links on a page** is IDM's "download all links": right-click an empty part of a page and choose **Download links on this page with Downpipe…**, or press **Links on this page…** in the popup's Downloads header. With text selected, the menu offers **Download selected links with Downpipe…**, which lists only the links inside the selection.

- The extension reads the page once, on that click, with `scripting.executeScript` and the `activeTab` grant the click gives it; it has no content scripts and no host permissions. The injected function only returns data: the page's address and title, and every `a[href]` and `area[href]` that is an http or https link, with its visible text (or its `title`, `aria-label` or image `alt`). `javascript:`, `mailto:`, `data:` and `blob:` links, in-page `#anchors` and links to the page itself are left out, each address is listed once (fragments ignored) and the list stops at 2,000 links. The service worker checks the result again before using it.
- The links open in a new tab next to the page. The service worker hands them over through `storage.session` under a random id that is part of the new tab's address; the links page validates what it reads and removes the entry, keeping a copy in its own tab's `sessionStorage` so a reload still shows the list. A collection nobody opened is dropped the next time a page is read, once it is ten minutes old, and session storage is cleared when the browser closes anyway.
- The links page shows the page's title and address, a text filter (every word must appear in the link text or address), **Hide web pages** (on by default: `html`, `htm`, `php`, `asp`, `aspx` and similar, and addresses without an extension), a chip for each of the twelve most common file extensions found plus **Other** for the rest and for addresses without one, **Select all** and **Select none** for the links on show, and a count. Links that are not web pages start selected. When every link is a web page, **Hide web pages** starts off so the list is not empty.
- **Download N links** sends the selected links that are on show, three at a time, each as its own `download` request with the page as `page_url` and the same referrer rule as the context menu. Each row then says **Added**, **In Downpipe** (already there) or **Failed** with the reason under the address, and a summary follows. Sent links are deselected, so pressing again retries only the failures. **Stop** lets the links already on their way finish and sends no more. The links also go into the popup's Downloads list, but without a notification or badge for each one.
- Before sending, the page checks the bridge. If the bridge is missing or refuses the extension, or Downpipe is not running and **Start Downpipe when it is closed** is off, nothing is sent and the page shows the same guidance as the popup. With the option on, the first link is sent alone and starts Downpipe, and the rest follow once it runs, so the links do not each start a copy. If the app goes away halfway, the page stops at the first such answer instead of failing every remaining link.
- Pages the browser does not let extensions read (its own settings pages, extension stores, some built-in viewers) open the links page with a note saying so.

**Media on this page** finds what a page plays, like IDM's "Download this video" button. It is off until you turn on its switch in the popup (shown on http and https pages). The browser then asks for two optional permissions, `webRequest` and access to all http and https sites; turning the switch off removes both and forgets every list.

- While it is on, the service worker looks at the media, XHR and fetch requests of every tab, the way IDM's download panel and open-source sniffers such as cat-catch do. It never blocks or changes a request and never reads a body. From each request it keeps only the `Referer` and `Origin` the page sent; from each response it reads the headers.
  - An HLS playlist (`.m3u8`, or a `mpegurl` type) or DASH manifest (`.mpd`, `application/dash+xml`) counts from any request.
  - An audio or video file counts when a `<video>` or `<audio>` element loaded it, or when a script fetched it whole: a range answer that names the file's total size, or a request that asks for a piece in its query string (`bytestart`, `byteend`, `range`, as some CDNs do), which is listed without those parameters so the whole file is downloaded. Players that fetch media themselves otherwise request fragments, which are never listed (`.ts`, `.m4s`, `video/mp2t`).
  - A file must be of a type in the media type list and at least the minimum size, both under **Filters → Media on pages** (by default the common video and audio types and 256 KiB). A file without an extension is judged by the type the server sends.
  - A stream is listed once, by its master playlist. A player loads the master and then the quality and audio playlists it names, often with random file names (`8k4_FZTUGeXpWu9v.m3u8`) and in the same folder, a sibling folder or on another host. So the service worker reads each new HLS playlist once itself, without cookies or the page's headers, at most 256 KiB and 5 seconds, and only its URI lines (see [Browser bridge](SECURITY_MODEL.md#browser-bridge) in the security model). A master's quality, audio and I-frame playlists, resolved against it, are remembered for the tab (up to 400) and never listed, and any of them already listed are removed: players sometimes fetch a quality playlist before the master, which then replaces it. Downpipe picks the quality from the master itself.
  - Playlists in a folder below a listed playlist of the same site (X's layout) count as its parts without being read, so they are also hidden when the master cannot be read (a playlist that needs the page's cookies, for one). A playlist the extension could not read is otherwise listed as before.
  - Nothing from a site on the excluded sites list is listed (the media's host, the frame's or the referrer's), and the Download button does not run there.
- The list is per tab, newest first, at most 20 entries, in session storage. It is cleared when the tab loads another page and when it closes. Pages that change their address without loading (single-page sites) keep adding to it.
- Entries are named after the tab's title, which is also what the download is named after; when several share it (a feed of videos), they are numbered from the oldest, `Home / X`, `Home / X (2)`. Without a title the file name is shown. Each entry's address is its tooltip in the popup.
- The popup's **Media on this page** lists the current tab's entries with their kind (`HLS · 3 qualities` for a master the extension read, otherwise `HLS stream`, `DASH stream`, `MP4 video` and so on), size (when the server said) and host, and a **Download** button each. If the video was already playing when detection was turned on, reload the page.
- A small **Download** button (an icon and a menu arrow) appears in the corner of a playing video that is at least 200 by 112 pixels, for a few seconds after it starts and while the pointer is over it; with **Show it only while the pointer is over the video** under Filters, only while the pointer is over it. It can be dragged anywhere inside the video, and the place (as fractions of the video's size) is remembered per site, for at most 100 sites, in `storage.local`. Its arrow opens **Hide on this site**, which adds the tab's site to **Sites without the button** under Filters: the content script is no longer injected there (those sites join its exclude patterns), and a player embedded from another site on such a page does not show the button either. Detection, the popup's list and capture go on on those sites. The button asks the service worker for these settings before it shows (`overlay_prefs`); hiding and saving the place are `overlay_hide_site` and `overlay_place`, which name no site: the service worker takes it from the sender's tab. It is a content script that the extension registers only while detection is on and **Show a Download button on videos** is on under Filters, with the excluded sites left out of its match patterns. A click asks the service worker for this tab's list (adding the video element's own source when it is a plain http(s) address); one entry is sent at once, several are offered in a menu. On a page with several videos, such as an X timeline or a post with a quoted video, the button sends the video's poster address along, and only the entries whose address shares an id with it are offered: X's poster `amplify_video_thumb/2098814416359653377/…` and playlist `amplify_video/2098814416359653377/pl/….m3u8` carry the same post id. Without a poster, or when nothing matches, the whole list is offered as before. The button ignores clicks that page scripts fake, and it does not appear on videos that use Encrypted Media Extensions (DRM), which Downpipe would refuse.
- Streams are sent as a `media` message, files as an ordinary `download` named after the page title. Both carry the page and the referrer the player's own request sent (if it sent none the extension computes one from the frame that requested the media, under the browser's default `strict-origin-when-cross-origin` policy). A stream also carries the `Origin` the player sent, since some servers check it. Never cookies or other headers: a stream that needs a signed-in session fails in Downpipe with the server's reason.
- In the app, the confirmation dialog of a stream offers its qualities and, when there is a choice, its audio tracks. It reads only the manifest for that. If FFmpeg is not available and video and sound come separately, the dialog says they will be saved as two files. A DRM-protected or live stream is reported there and can only be removed.
- With an app or bridge from before streams, the `pong` has no `media` feature: the popup marks streams **Update Downpipe to download streams** (files still work), and a stream sent anyway is refused with that message.

**Video pages (YouTube)** go to the app's site provider instead, which runs [yt-dlp](https://github.com/yt-dlp/yt-dlp). YouTube's player no longer fetches files a download manager could take over: it streams over SABR and its links need tokens and answers to JavaScript challenges that only an extractor like yt-dlp produces. So on such a page the extension sends the page's address, not the media requests it sees.

- The pages are listed in `apps/browser-extension/src/lib/sites.ts` and, with the same test cases (`crates/providers/tests/fixtures/pages.json`), in `crates/providers/src/pages.rs`: `youtube.com/watch?v=…`, `/shorts/…`, `/embed/…`, `/live/…`, `m.youtube.com`, `music.youtube.com`, `youtube-nocookie.com/embed/…` and `youtu.be/…`. Playlists, channels and search pages are not video pages. The link is sent as `https://www.youtube.com/watch?v=<id>`, without timestamps, playlists or tracking, so the same video sent twice is a duplicate.
- The popup shows the page as the first row of **Media on this page**, titled after the tab, with **Download**. This works with media detection off: opening the popup grants `activeTab`, which is enough to read the tab's address.
- With detection on, the Download button over the playing video offers the page as well (on an embedded player, the embed's video). The requests YouTube's player makes to its video servers (`*.googlevideo.com`) are never listed, since they cannot be downloaded on their own.
- **Download this video with Downpipe** in the page's context menu on YouTube, and **Download with Downpipe** on a link to a video page, send the page the same way.
- The message is `page`: the page address and the tab's title (without " - YouTube"), nothing else. No cookies: yt-dlp in the app reads the page signed out, so private, members-only and age-restricted videos fail with YouTube's reason.
- In the app, the confirmation dialog asks yt-dlp for the formats (`yt-dlp -J`, which fetches the page and its player data, no video) and offers them in the same **Quality** and **Audio** lists as a stream: one entry per resolution and frame rate, the separate sound tracks, and **Audio only**. Protected and live videos are reported there and can only be removed.
- With an app or bridge from before the site provider, `pong` has no `page` feature: the popup's page row says **Update Downpipe to download videos from pages**, and a page sent anyway is refused with that message.
- **Playlists and channels.** On a YouTube playlist (`playlist?list=...`) or channel (`@handle`, `channel/UC...`, `c/...`, `user/...`, with their Videos, Shorts and Live tabs), the popup's page row reads **YouTube playlist · videos chosen in Downpipe** (or channel), the page's context menu offers **Choose videos from this list with Downpipe…**, and **Download with Downpipe** on a link to one does the same. The extension sends a `page` message with the list's address; the app answers `list_opened` and brings its main window up with the list of the videos, where they are chosen (see the app's playlist dialog below). Nothing is added before that. Watch later, Liked videos and Liked music are not offered: without the browser's cookies they are empty. A video link that also names a playlist (`watch?v=...&list=...`) is that one video.
- With an app or bridge from before lists, `pong` has no `list` feature and the row says **Update Downpipe to download playlists**.

While **Ask before downloads from the browser start** is on in the app (Settings → App, on by default), the app still answers `accepted` at once, so the browser never waits for a person. The download is created paused and a small confirmation window comes up over the browser, like IDM's Download File Info, asking for its file name, folder and queue (the main window stays as it was, in the tray or not): **Start download** queues it, **Download later** keeps it paused, and **Cancel** removes it. Nothing is requested from the server before the answer. The `accepted` answer says so with `awaiting_confirmation: true`, so the extension's notification reads "Waiting for your answer in Downpipe" instead of "Downloading in Downpipe". Until the dialog is answered the popup lists the download as **Waiting in Downpipe** (not Paused) and keeps asking the app every second, so the row turns into Downloading, or Paused after **Download later**, while the popup is open. After **Cancel** the app no longer knows the download, and the row falls back to what was recorded when it was sent.

Every hand-over from the link menu or capture is confirmed in three places:

- **A notification:** "Downloading in Downpipe" with the file name, or "Waiting for your answer in Downpipe" when the app asks first. When Downpipe could not take a download, the notification says so and stays until dismissed, for example "Downloaded by the browser" when the app was closed. **Notifications** in the popup turns them off.
- **The toolbar badge:** `+` for a few seconds after a link was added, `!` when something went wrong.
- **The popup's Downloads list:** the last five downloads with their live state from the app (Queued, Downloading, Done, Failed and so on). A running download shows percent, size and speed, and the list refreshes every second while the popup is open. Downloads Downpipe did not take are marked "Not sent" or "In browser" with the reason.

The list lives in the browser's session storage and is cleared when the browser closes. The popup asks the app only about these downloads, by the ids the app returned, never for the rest of the download list.

## Registering by hand

`downpipe-native-host manifest --browser chrome` prints the manifest instead of installing it. For Chrome, Edge and Chromium:

```json
{
  "name": "dev.downpipe.native_host",
  "description": "Downpipe browser bridge",
  "path": "C:\\Program Files\\Downpipe\\downpipe-native-host.exe",
  "type": "stdio",
  "allowed_origins": ["chrome-extension://efgicfboaldflckpncfdojkeoaamflpf/"]
}
```

Firefox uses `"allowed_extensions": ["downpipe@downpipe.dev"]` instead of `allowed_origins`. Browsers do not accept wildcards in either list; every allowed extension id is written out. To allow another id (a store build, or an unpacked copy with a different key), pass `--extension-id <id>` or `--firefox-extension-id <id>` to `install` or `manifest`; repeat the flag for several ids.

**Windows.** The manifest can live anywhere; `install` writes it to `%LOCALAPPDATA%\Downpipe\NativeMessagingHosts\<browser>\dev.downpipe.native_host.json`. The browser finds it through the default value of a registry key under `HKEY_CURRENT_USER`:

| Browser | Key |
|---|---|
| Chrome | `HKCU\Software\Google\Chrome\NativeMessagingHosts\dev.downpipe.native_host` |
| Edge | `HKCU\Software\Microsoft\Edge\NativeMessagingHosts\dev.downpipe.native_host` |
| Chromium | `HKCU\Software\Chromium\NativeMessagingHosts\dev.downpipe.native_host` |
| Firefox | `HKCU\Software\Mozilla\NativeMessagingHosts\dev.downpipe.native_host` |

The same registration by hand, for Chrome:

```powershell
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\dev.downpipe.native_host" /ve /t REG_SZ /d "C:\path\to\dev.downpipe.native_host.json" /f
```

**macOS.** The manifest file itself goes in the browser's folder, with `path` set to the absolute path of the host:

| Browser | Folder |
|---|---|
| Chrome | `~/Library/Application Support/Google/Chrome/NativeMessagingHosts/` |
| Edge | `~/Library/Application Support/Microsoft Edge/NativeMessagingHosts/` |
| Chromium | `~/Library/Application Support/Chromium/NativeMessagingHosts/` |
| Firefox | `~/Library/Application Support/Mozilla/NativeMessagingHosts/` |

**Linux.**

| Browser | Folder |
|---|---|
| Chrome | `~/.config/google-chrome/NativeMessagingHosts/` |
| Edge | `~/.config/microsoft-edge/NativeMessagingHosts/` |
| Chromium | `~/.config/chromium/NativeMessagingHosts/` |
| Firefox | `~/.mozilla/native-messaging-hosts/` |

The file name is `dev.downpipe.native_host.json` everywhere. Without `--browser`, `install` on macOS and Linux registers only the browsers whose profile folder exists.

## Protocol, version 1

Each message is a 32-bit length in native byte order followed by that many bytes of UTF-8 JSON (Chrome's Native Messaging framing). The host reads messages until the browser closes stdin and answers each one in order.

Requests:

```json
{"protocol_version": 1, "type": "ping", "id": "p1"}
{"protocol_version": 1, "type": "download", "id": "m1",
 "resource_url": "https://cdn.example.net/files/report.pdf",
 "page_url": "https://example.org/downloads",
 "referrer": "https://example.org/",
 "mime_type": "application/pdf",
 "file_name": "report.pdf"}
{"protocol_version": 1, "type": "media", "id": "s1",
 "manifest_url": "https://video.example.net/streams/clip/master.m3u8",
 "kind": "hls",
 "page_url": "https://example.org/watch/clip",
 "referrer": "https://example.org/",
 "origin": "https://example.org",
 "title": "Clip of the week"}
{"protocol_version": 1, "type": "page", "id": "y1",
 "page_url": "https://www.youtube.com/watch?v=jNQXAC9IVRw",
 "title": "Me at the zoo"}
{"protocol_version": 1, "type": "page", "id": "y2",
 "page_url": "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb"}
{"protocol_version": 1, "type": "tasks", "id": "t1",
 "task_ids": ["8e48dd86-5481-41af-a2fc-edf3044e29b8"]}
{"protocol_version": 1, "type": "download", "id": "m3",
 "resource_url": "https://example.org/files/report.pdf",
 "file_name": "report.pdf", "start_app": false}
```

| Field | Rules |
|---|---|
| `protocol_version` | Required, must be `1`. Other numbers get `unsupported_version`. |
| `type` | `ping`, `download`, `media`, `page` or `tasks`. |
| `id` | Optional, 1 to 64 letters, digits, `-` or `_`. Echoed in the response. |
| `resource_url` | Required for `download`. http or https, at most 8 KiB, no control characters. The fragment is dropped. |
| `page_url`, `referrer` | Optional, for `download` and `media`, same URL rules. Credentials and fragments are removed. The referrer is sent as `Referer` (with a stream: with the manifest, every fragment and the key); the page URL is not stored. For `page`, `page_url` is required and is what is downloaded: it must be a video page the site provider takes, or a playlist or channel it lists (see above), or the answer is `unsupported_page`; a `page` message carries no referrer. |
| `manifest_url` | Required for `media`, allowed nowhere else. The HLS playlist or DASH manifest, with the same rules as `resource_url`. |
| `kind` | Required for `media`: `hls` or `dash`. |
| `title` | Optional, for `media` and `page`: the page title, at most 200 characters, no control characters. The app names the file after it. |
| `origin` | Optional, for `media`: the `Origin` the player sent, exactly `scheme://host[:port]`. Sent as `Origin` with the manifest and every fragment. |
| `mime_type` | Optional, reduced to `type/subtype`. |
| `file_name` | Optional hint, at most 255 characters, no folders and no control characters. The app sanitizes it like a server-supplied name. |
| `task_ids` | Required for `tasks`, allowed nowhere else. 1 to 20 ids as returned in `accepted` (or `duplicate`). |
| `start_app` | Optional boolean, for `download`, `media` and `page`; `ping` and `tasks` refuse it. Missing or `true`: the host starts a closed app first. `false`: it answers `app_not_running` instead. |

`resource_url`, `mime_type` and `file_name` are `download` fields and `manifest_url`, `kind`, `origin` and `title` are `media` fields; each message type refuses the other's. A `page` message carries only `page_url` and `title`. Unknown fields, duplicate fields and any field whose name contains `cookie` are rejected. A message over 64 KiB is answered with `message_too_large`, after which the host exits, because the rest of the stream can no longer be framed reliably.

Responses carry `protocol_version`, the request's `id` and a `type`:

```json
{"protocol_version": 1, "id": "p1", "type": "pong", "host_version": "0.4.0", "app_running": true, "app_version": "0.4.0", "features": ["media", "page", "list"]}
{"protocol_version": 1, "id": "m1", "type": "accepted", "task_id": "8e48dd86-..."}
{"protocol_version": 1, "id": "m2", "type": "accepted", "task_id": "68b6ffff-...", "awaiting_confirmation": true}
{"protocol_version": 1, "id": "y2", "type": "list_opened", "page_url": "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb"}
{"protocol_version": 1, "id": "m1", "type": "error", "code": "duplicate", "message": "This link is already in Downpipe", "task_id": "8e48dd86-..."}
{"protocol_version": 1, "id": "t1", "type": "tasks", "tasks": [
  {"task_id": "8e48dd86-...", "state": "downloading", "file_name": "report.pdf",
   "downloaded": 3145728, "total": 12582912, "speed": 1310720},
  {"task_id": "68b6ffff-...", "state": "paused", "downloaded": 0, "speed": 0,
   "awaiting_confirmation": true}]}
```

| Field | In | Meaning |
|---|---|---|
| `features` | `pong` | What the host and the running app both support beyond the first messages of version 1. `media`: the `media` message; `page`: the `page` message; `list`: `page` also takes playlists and channels. Left out when there is none (the app is closed, or one of them predates it). |
| `task_id` | `accepted`, `tasks`, `duplicate` errors | The app's id for the download. |
| `file_name` | `accepted`, `tasks` | The hint or the name the app chose; in `tasks`, the final name once the server has answered. |
| `state` | `tasks` | One of `queued`, `starting`, `downloading`, `waiting`, `verifying`, `paused`, `completed`, `failed` and `canceled`. |
| `downloaded`, `total`, `speed` | `tasks` | Bytes so far, bytes in all (missing while the size is unknown), bytes per second. |
| `error` | `tasks` | At most 300 characters explaining a failure, in English. |
| `error_kind` | `tasks` | The failure's stable kind (`not_found`, `network`, `expired_link`, ...: `DownloadError::kind()`), so the extension words it in the browser's language. Left out when unknown; apps before it never send it. |
| `page_url` | `list_opened` | The list's own page (a channel's videos tab for a channel link). The app's main window lists its videos; nothing is added yet. |
| `awaiting_confirmation` | `accepted`, `tasks` | Sent only as `true`: the app holds the download, paused, until the user answers its confirmation dialog. In `tasks` it comes with `state: "paused"`. |

Ids the app does not know in a `tasks` query, for example removed downloads, are left out of the answer.

Messages may also gain types within version 1, as `media` and `page` did: a host or app that predates one answers `invalid_message` ("unknown message type"), which an extension can tell from a broken bridge, and `pong` lists the ones both ends know in `features`, so the extension can check before sending. The pipe between host and app gained the `media` and `page` actions the same way, without a new `ipc_version`, since older actions are unchanged.

Requests gained one optional field the same way, `start_app`. The extension sends it only as `false`, when its option is off; a host from before it refuses the unknown field with `invalid_message`, and since such a host never starts the app, the extension then sends the message again without it.

Answers may gain optional fields within version 1; the extension ignores fields it does not know, and the host and the app accept each other's answers with or without them. That is why the confirmation is a flag and not a tenth `state`: the download really is paused in the app's catalog, and extensions from before the flag drop task entries with a state outside their fixed list, which would cost them that row's live state. With the flag they keep showing Paused, which is still true.

| Error code | Meaning |
|---|---|
| `invalid_message` | Not JSON, wrong shape, unknown or duplicate field, bad value. |
| `invalid_url` | A URL is not http/https, too long or malformed. |
| `cookies_not_accepted` | The message tried to carry cookies. |
| `unsupported_version` | `protocol_version` is not 1. |
| `message_too_large` | Over 64 KiB. |
| `app_not_running` | The host could not find the app's pipe or socket, and did not start the app: `start_app` was `false`, there is no app next to the host, it could not be run, or it did not open its pipe within 10 seconds. |
| `app_unavailable` | The app did not answer within 10 seconds or answered something unexpected. |
| `version_mismatch` | The host and the app speak different pipe versions; run `install` from the same build as the app. |
| `duplicate` | An unfinished download with this URL already exists; `task_id` names it. |
| `unsupported_page` | A `page` message for a site Downpipe has no provider for. |

Other codes come from the download queue (for example `invalid_header`) and carry a readable `message`. The extension words the bridge's codes above and the engine's failure kinds itself, in the browser's language, and shows the `message` only for codes it does not know.

The extension and the Rust tests read the same example messages from `crates/native-host/tests/fixtures`, so the two sides cannot drift apart unnoticed.

## Host and app

The host opens one connection per message, sends `{"ipc_version": 1, "action": ...}` with the same framing and waits up to 10 seconds for the answer. The app checks every request again before calling `Manager::add`, because any program running as the same user can open the pipe.

`DOWNPIPE_BRIDGE_ENDPOINT` overrides the pipe name or socket path for both sides; the end-to-end tests use it so they never touch a running Downpipe. The app logs `browser bridge listening on ...` at start-up, or `browser bridge is off: ...` if another process already serves the endpoint.

Starting Downpipe while it is already running brings the open window forward instead of starting a second copy, so the bridge stays with the first one. Copies started with their own `DOWNPIPE_DATA_DIR` (tests, portable setups, a development build next to an installed Downpipe) run side by side on purpose; the later one logs `browser bridge is off` unless it also gets its own `DOWNPIPE_BRIDGE_ENDPOINT`.

### Starting the app

When a `download`, `media` or `page` message finds no app and `start_app` allows it, the host starts the desktop app and delivers the message once the app answers:

- **Which program.** Only an executable in the host's own folder, under a fixed name: `Downpipe.exe`, `downpipe.exe` or `downpipe-desktop.exe` on Windows (`Downpipe`, `downpipe` or `downpipe-desktop` elsewhere), the names the installers and Cargo give the app. The installers put the host next to the app, and a development build has both in Cargo's target folder. Nothing in the message takes part in choosing it, and a host without an app beside it (a copy run by hand, the AppImage's registration) starts nothing.
- **How.** With `--hidden`, so Downpipe opens in the tray like the login item starts it, with no inherited standard streams (stdout carries the protocol), in its own process group, and on Windows outside the browser's job object where the browser allows it, so closing the browser does not close Downpipe.
- **Waiting.** The host asks the pipe every 150 ms with a `ping` for up to 10 seconds and then sends the message. A message that found no pipe never reached the app, so sending it after the start cannot add it twice. If the app cannot be run or does not answer in time, the answer is `app_not_running`, as it was before the host could start it, and a captured download stays with the browser. The extension waits 12 seconds longer for these answers while the option is on.
- **Several at once.** Two messages arriving together may both start the app; the second copy hands over to the first (single instance) and exits, and `--hidden` keeps it from opening the window.

## Troubleshooting

| Popup says | Do this |
|---|---|
| Browser bridge not installed | Start Downpipe; it registers the bridge for your user account. **Settings → Browser bridge** shows the state for each browser. With the start-up registration off, use **Register now** there or run `downpipe-native-host install`. |
| Bridge does not allow this extension | The extension has a different id than the manifest allows. Run the command the popup shows; it includes this extension's id. The app keeps that id when it rewrites the manifest later. |
| Downpipe is not running | Start the app. If it is running, check its log for `browser bridge is off`. A download sent from the browser starts Downpipe by itself unless **Start Downpipe when it is closed** is off on the Filters page, or the registered host has no app next to it (`downpipe-native-host status` shows the host). |
| Bridge error | Restart Downpipe or use **Register now** so the browsers start the host that came with this version of the app. `downpipe-native-host status` shows what is registered. |
| Update Downpipe to download streams | The app, or the bridge it registered, is older than the extension. Update Downpipe and start it once. |
| Media on this page stays empty | Play the video. If it was already playing when detection was turned on, reload the page. Videos that use DRM are never listed. |

The host writes diagnostics to stderr, which Chrome includes in its log when started with `--enable-logging --v=1`. `DOWNPIPE_LOG=debug` makes both the host and the app more verbose.
