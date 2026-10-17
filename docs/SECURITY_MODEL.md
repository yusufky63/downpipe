# Security model

A download manager handles untrusted data from servers and, through the browser extension, from web pages. This page lists what Downpipe trusts, what it does not, and how each boundary is enforced today.

## Untrusted inputs and how they are handled

| Input | Risk | Handling |
|---|---|---|
| `Content-Disposition` and URL file names | Path traversal (`../`), absolute paths, Windows device names (`CON`, `COM1`), reserved characters, hidden extensions via trailing dots | `filename::sanitize`: keeps only the last path component, replaces `<>:"/\|?*` and control characters, strips trailing dots and spaces, prefixes device names, caps length at 200 bytes while keeping the extension. Existing files are never overwritten. |
| Range responses | A server that ignores ranges or changes the file mid-download produces a corrupt file | Every follow-up `206` must match the first offset, total size and ETag; a `200` to a range request is rejected; changes restart the download from zero. |
| Server-sent bytes beyond the announced size | Writing past the end | Writes are clipped to the segment and the announced size. |
| `Retry-After` | A huge value parks a download forever | Capped at 10 minutes. |
| Redirects | Loops | At most 10 redirects. Follow-up requests use the final URL of the current session; resumes go through the original URL again because redirect targets often expire. |
| TLS | Interception | Certificate validation is always on (rustls with the platform verifier). There is no option to turn it off. |
| Messages from the browser extension | A compromised or buggy extension, or any local program that opens the pipe, sending crafted requests | Fixed, versioned schema checked by both the native host and the app (see [Browser bridge](#browser-bridge)). |

## Secrets

- `user:password@host` in a URL is used for HTTP Basic auth by the HTTP client, but URLs are stored with credentials removed (`redact_url`). Logs use `display_url`, which also drops the query string, because signed URLs carry tokens there.
- Custom request headers (for example `Referer`) are kept in memory only and never written to the meta file or the database. After a restart such downloads continue without them.
- Error messages shown in the UI are built from error classes, not from the HTTP client's display string, which can contain the full URL.
- Downpipe does not read browser cookies, and the bridge refuses any message that tries to carry them. A future opt-in, per-site cookie feature is described in the roadmap and will never log or store cookie values.

## Desktop app

- The web view runs local, bundled content under a strict CSP (`script-src 'self'`, no remote origins).
- It holds only Tauri's `core:default` capability. Folder pickers and "open" / "show in folder" run in Rust, and those commands take a download id, not a path, so a compromised web view still cannot open arbitrary files.
- No network port is opened, including for the browser bridge.

## Integrity

- Data reaches its final name only through an atomic rename after the last byte is written and, if requested, the checksum matches.
- Resume metadata is written with write-to-temp-then-rename after the data it describes has been synced.

## Browser bridge

The path is extension, then Native Messaging to `downpipe-native-host`, then a local pipe to the app. Details in [BROWSER_BRIDGE.md](BROWSER_BRIDGE.md).

**Who may start the host.** Browsers start a Native Messaging host only for extensions its manifest lists, and they do not accept wildcards. `install`, and the desktop app at start-up, write the Downpipe extension's id explicitly (`allowed_origins` for Chrome, Edge and Chromium, `allowed_extensions` for Firefox). The Chrome manifest carries a public key so that unpacked builds get that same id. Registration is per user (`HKEY_CURRENT_USER`, or files in the user's home), so no administrator rights are involved. The app only ever points the registration at the host next to its own executable and keeps, but never adds, other extension ids; it does not read or write `HKEY_LOCAL_MACHINE` or system-wide manifest folders.

**Extension permissions.** `contextMenus`, `nativeMessaging`, `notifications` and `storage`, and `downloads` only while download capture is switched on (an optional permission the popup requests and removes). No host permissions, no `<all_urls>`, no content scripts. The extension never reads page content or cookies.

**Inside the extension.** The service worker answers only messages whose sender is this extension's own pages: the sender id must match and its URL and origin must be the extension's base URL. It registers no `onMessageExternal` listener, so other extensions and web pages cannot drive it. Strings from the host are length-capped and written into the popup with `textContent`, never as HTML.

**Messages.** Requests have a fixed, versioned schema (`protocol_version: 1`). The host rejects unknown or duplicate fields, any field whose name mentions cookies, non-http(s) URLs (`javascript:`, `file:`, `data:`, `blob:` and so on), control characters (which URL parsers would silently drop), URLs over 8 KiB, file name hints with folders, and frames over 64 KiB, before allocating the frame. Referrers and page URLs lose credentials and fragments; the referrer the extension computes follows the browser's default `strict-origin-when-cross-origin` policy, so a cross-site link gets only the page's origin.

**The pipe.** On Windows the app creates `\\.\pipe\downpipe-bridge-<user>` as the first instance (so it fails rather than joins a pipe someone else created), refuses remote clients, and keeps the default security descriptor: write access only for the user, SYSTEM and administrators, and a medium integrity label that keeps sandboxed, low-integrity processes such as browser renderers out. The host opens the pipe with `SECURITY_IDENTIFICATION`, so even a process that squatted the name could not impersonate the user. On macOS and Linux the socket lives in a folder created with mode `0700` (the app refuses a folder others can enter) and the socket itself is `0600`.

**The app trusts nothing on the pipe.** Any program running as the same user can connect, so the app validates every request again with the same code, accepts one request per connection, drops idle connections after five seconds, and only ever calls `Manager::add`. A request cannot choose the download folder, pass headers other than the referrer, or touch existing downloads. Progress queries (`tasks`) take at most 20 ids and answer only for those ids, so the extension cannot list the user's other downloads.

**Privacy.** The app stores the download URL (without credentials) as it does for any download; the page URL is not stored. Private-window downloads are never captured. The popup's activity list is kept in session storage and cleared when the browser closes.

**Known limits.** A process running as the same user can add downloads through the pipe, the same as it could by driving the app's window. On Windows another account could claim the pipe name before Downpipe starts; it would then receive the URLs the host sends, though not the user's credentials or token. Both are within the "same user / same machine" trust boundary this model assumes.

## Planned boundaries

- **External providers (yt-dlp, FFmpeg):** started with argument arrays, never shell strings; their JSON output is validated against a schema before use; DRM-protected media is reported as unsupported.

## Reporting a vulnerability

See [SECURITY.md](../SECURITY.md).
