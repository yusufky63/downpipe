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

**Capture browser downloads** (off by default) is a switch in the popup. Turning it on asks the browser for the `downloads` permission; turning it off removes the permission again. While it is on:

- A download the browser starts is offered to Downpipe. If the app accepts it (or already has it), the browser's copy is cancelled and removed from the browser's download list.
- If Downpipe is not running or refuses it, the browser keeps downloading as if the extension were not there, and the popup notes that the browser kept the file.
- Always skipped: downloads in private windows (the app keeps a history of its downloads), `blob:` and `data:` URLs (they only exist inside the browser), and downloads the extension started itself.
- Then the capture filters below decide. A download they leave out is never touched: the browser downloads it as usual, with no notification.
- Chrome and Edge decide in `downloads.onDeterminingFilename`, after the server's file name, type and usually its size are known. Firefox has no such event; there the filters are checked in `downloads.onCreated`, and a download they let through is paused, offered to Downpipe and resumed if Downpipe does not take it.

Downpipe never receives browser cookies. Downloads that only work while you are signed in to a site, and downloads that come from submitting a form, fail in Downpipe. Pause capture for those sites, or let the browser download them; a per-site cookie opt-in is on the roadmap.

**Capture filters** opens from the link under the capture switch in the popup (it is also the extension's options page). It works like IDM's file type settings. Nothing changes until **Save** is pressed; invalid entries are pointed out under their field and nothing is saved until they are fixed.

- **File types.** Only downloads whose file name ends in one of the listed extensions are captured. The list is separated by spaces or commas; `.zip`, `*.zip` and `ZIP` all mean `zip`, and compound extensions such as `tar.gz` work. The name is the browser's final file name (in Chrome and Edge it already includes the server's `Content-Disposition` name); when the browser has none, the last segment of the URL path, so `get.php?file=a.zip` counts as `php`. The default list covers archives (`zip`, `7z`, `rar`, `tar`, `gz`, `xz` and more), programs and packages (`exe`, `msi`, `apk`, `dmg`, `pkg`, `deb`, `rpm`, `appimage`), disk images (`iso`, `img`, `bin`, `vhd`), documents (`pdf`, `epub`, Office and OpenDocument files), audio (`mp3`, `m4a`, `flac`, `ogg`, `wav` and more) and video (`mp4`, `mkv`, `webm`, `mov`, `avi` and more). Web pages, images and scripts stay in the browser. **Capture all file types** ignores the list (and keeps it for later); **Restore the default list** brings the defaults back.
- **Excluded sites.** One host per line. `example.com` matches only that host; `*.example.com` matches `example.com` and all of its subdomains. A download is left to the browser when its URL, the address it was redirected to, or the page it started on (the referrer) is on one of these sites. Matching ignores case, ports and paths, and pasted URLs are reduced to their host. International names are stored in their ASCII (punycode) form, which is how URLs carry them, so `bücher.de` is saved as `xn--bcher-kva.de`.
- **Minimum size.** Downloads smaller than this, in KiB or MiB, stay in the browser. 0 turns it off. When the server does not send a size, the browser reports it as unknown, and such downloads are always offered to Downpipe.

The filters are stored with the other extension settings in `storage.local`. Settings saved by an earlier version get the defaults for the new fields.

**Pause capture on this site** appears under the capture switch while capture is on and the current tab is an http or https page. It adds the tab's host to the excluded sites; **Resume capture on this site** removes that entry again. A wildcard entry that already covers the site is shown instead of the button, since removing it would affect other sites; edit it under Capture filters. The popup reads the tab's address through `activeTab`, which the browser grants when the popup is opened.

**Links on a page** is IDM's "download all links": right-click an empty part of a page and choose **Download links on this page with Downpipe…**, or press **Links on this page…** in the popup's Downloads header. With text selected, the menu offers **Download selected links with Downpipe…**, which lists only the links inside the selection.

- The extension reads the page once, on that click, with `scripting.executeScript` and the `activeTab` grant the click gives it; it has no content scripts and no host permissions. The injected function only returns data: the page's address and title, and every `a[href]` and `area[href]` that is an http or https link, with its visible text (or its `title`, `aria-label` or image `alt`). `javascript:`, `mailto:`, `data:` and `blob:` links, in-page `#anchors` and links to the page itself are left out, each address is listed once (fragments ignored) and the list stops at 2,000 links. The service worker checks the result again before using it.
- The links open in a new tab next to the page. The service worker hands them over through `storage.session` under a random id that is part of the new tab's address; the links page validates what it reads and removes the entry, keeping a copy in its own tab's `sessionStorage` so a reload still shows the list. A collection nobody opened is dropped the next time a page is read, once it is ten minutes old, and session storage is cleared when the browser closes anyway.
- The links page shows the page's title and address, a text filter (every word must appear in the link text or address), **Hide web pages** (on by default: `html`, `htm`, `php`, `asp`, `aspx` and similar, and addresses without an extension), a chip for each of the twelve most common file extensions found plus **Other** for the rest and for addresses without one, **Select all** and **Select none** for the links on show, and a count. Links that are not web pages start selected. When every link is a web page, **Hide web pages** starts off so the list is not empty.
- **Download N links** sends the selected links that are on show, three at a time, each as its own `download` request with the page as `page_url` and the same referrer rule as the context menu. Each row then says **Added**, **In Downpipe** (already there) or **Failed** with the reason under the address, and a summary follows. Sent links are deselected, so pressing again retries only the failures. **Stop** lets the links already on their way finish and sends no more. The links also go into the popup's Downloads list, but without a notification or badge for each one.
- Before sending, the page checks the bridge. If Downpipe is not running, or the bridge is missing or refuses the extension, nothing is sent and the page shows the same guidance as the popup. If the app goes away halfway, the page stops at the first such answer instead of failing every remaining link.
- Pages the browser does not let extensions read (its own settings pages, extension stores, some built-in viewers) open the links page with a note saying so.

While **Ask before downloads from the browser start** is on in the app (Settings → App, on by default), the app still answers `accepted` at once, so the browser never waits for a person. The download is created paused and the app's window comes forward with a dialog for its file name, folder and queue: **Start download** queues it, **Download later** keeps it paused, and **Cancel** removes it. Nothing is requested from the server before the answer. The `accepted` answer says so with `awaiting_confirmation: true`, so the extension's notification reads "Waiting for your answer in Downpipe" instead of "Downloading in Downpipe". Until the dialog is answered the popup lists the download as **Waiting in Downpipe** (not Paused) and keeps asking the app every second, so the row turns into Downloading, or Paused after **Download later**, while the popup is open. After **Cancel** the app no longer knows the download, and the row falls back to what was recorded when it was sent.

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
{"protocol_version": 1, "type": "tasks", "id": "t1",
 "task_ids": ["8e48dd86-5481-41af-a2fc-edf3044e29b8"]}
```

| Field | Rules |
|---|---|
| `protocol_version` | Required, must be `1`. Other numbers get `unsupported_version`. |
| `type` | `ping`, `download` or `tasks`. |
| `id` | Optional, 1 to 64 letters, digits, `-` or `_`. Echoed in the response. |
| `resource_url` | Required for `download`. http or https, at most 8 KiB, no control characters. The fragment is dropped. |
| `page_url`, `referrer` | Optional, same URL rules. Credentials and fragments are removed. The referrer is sent as `Referer`; the page URL is not stored. |
| `mime_type` | Optional, reduced to `type/subtype`. |
| `file_name` | Optional hint, at most 255 characters, no folders and no control characters. The app sanitizes it like a server-supplied name. |
| `task_ids` | Required for `tasks`, allowed nowhere else. 1 to 20 ids as returned in `accepted` (or `duplicate`). |

Unknown fields, duplicate fields and any field whose name contains `cookie` are rejected. A message over 64 KiB is answered with `message_too_large`, after which the host exits, because the rest of the stream can no longer be framed reliably.

Responses carry `protocol_version`, the request's `id` and a `type`:

```json
{"protocol_version": 1, "id": "p1", "type": "pong", "host_version": "0.3.0", "app_running": true, "app_version": "0.3.0"}
{"protocol_version": 1, "id": "m1", "type": "accepted", "task_id": "8e48dd86-..."}
{"protocol_version": 1, "id": "m2", "type": "accepted", "task_id": "68b6ffff-...", "awaiting_confirmation": true}
{"protocol_version": 1, "id": "m1", "type": "error", "code": "duplicate", "message": "This link is already in Downpipe", "task_id": "8e48dd86-..."}
{"protocol_version": 1, "id": "t1", "type": "tasks", "tasks": [
  {"task_id": "8e48dd86-...", "state": "downloading", "file_name": "report.pdf",
   "downloaded": 3145728, "total": 12582912, "speed": 1310720},
  {"task_id": "68b6ffff-...", "state": "paused", "downloaded": 0, "speed": 0,
   "awaiting_confirmation": true}]}
```

| Field | In | Meaning |
|---|---|---|
| `task_id` | `accepted`, `tasks`, `duplicate` errors | The app's id for the download. |
| `file_name` | `accepted`, `tasks` | The hint or the name the app chose; in `tasks`, the final name once the server has answered. |
| `state` | `tasks` | One of `queued`, `starting`, `downloading`, `waiting`, `verifying`, `paused`, `completed`, `failed` and `canceled`. |
| `downloaded`, `total`, `speed` | `tasks` | Bytes so far, bytes in all (missing while the size is unknown), bytes per second. |
| `error` | `tasks` | At most 300 characters explaining a failure. |
| `awaiting_confirmation` | `accepted`, `tasks` | Sent only as `true`: the app holds the download, paused, until the user answers its confirmation dialog. In `tasks` it comes with `state: "paused"`. |

Ids the app does not know in a `tasks` query, for example removed downloads, are left out of the answer.

Answers may gain optional fields within version 1; the extension ignores fields it does not know, and the host and the app accept each other's answers with or without them. That is why the confirmation is a flag and not a tenth `state`: the download really is paused in the app's catalog, and extensions from before the flag drop task entries with a state outside their fixed list, which would cost them that row's live state. With the flag they keep showing Paused, which is still true.

| Error code | Meaning |
|---|---|
| `invalid_message` | Not JSON, wrong shape, unknown or duplicate field, bad value. |
| `invalid_url` | A URL is not http/https, too long or malformed. |
| `cookies_not_accepted` | The message tried to carry cookies. |
| `unsupported_version` | `protocol_version` is not 1. |
| `message_too_large` | Over 64 KiB. |
| `app_not_running` | The host could not find the app's pipe or socket. |
| `app_unavailable` | The app did not answer within 10 seconds or answered something unexpected. |
| `version_mismatch` | The host and the app speak different pipe versions; run `install` from the same build as the app. |
| `duplicate` | An unfinished download with this URL already exists; `task_id` names it. |

Other codes come from the download queue (for example `invalid_header`) and carry a readable `message`.

The extension and the Rust tests read the same example messages from `crates/native-host/tests/fixtures`, so the two sides cannot drift apart unnoticed.

## Host and app

The host opens one connection per message, sends `{"ipc_version": 1, "action": ...}` with the same framing and waits up to 10 seconds for the answer. The app checks every request again before calling `Manager::add`, because any program running as the same user can open the pipe.

`DOWNPIPE_BRIDGE_ENDPOINT` overrides the pipe name or socket path for both sides; the end-to-end tests use it so they never touch a running Downpipe. The app logs `browser bridge listening on ...` at start-up, or `browser bridge is off: ...` if another process already serves the endpoint.

Starting Downpipe while it is already running brings the open window forward instead of starting a second copy, so the bridge stays with the first one. Copies started with their own `DOWNPIPE_DATA_DIR` (tests, portable setups, a development build next to an installed Downpipe) run side by side on purpose; the later one logs `browser bridge is off` unless it also gets its own `DOWNPIPE_BRIDGE_ENDPOINT`.

## Troubleshooting

| Popup says | Do this |
|---|---|
| Browser bridge not installed | Start Downpipe; it registers the bridge for your user account. **Settings → Browser bridge** shows the state for each browser. With the start-up registration off, use **Register now** there or run `downpipe-native-host install`. |
| Bridge does not allow this extension | The extension has a different id than the manifest allows. Run the command the popup shows; it includes this extension's id. The app keeps that id when it rewrites the manifest later. |
| Downpipe is not running | Start the app. If it is running, check its log for `browser bridge is off`. |
| Bridge error | Restart Downpipe or use **Register now** so the browsers start the host that came with this version of the app. `downpipe-native-host status` shows what is registered. |

The host writes diagnostics to stderr, which Chrome includes in its log when started with `--enable-logging --v=1`. `DOWNPIPE_LOG=debug` makes both the host and the app more verbose.
