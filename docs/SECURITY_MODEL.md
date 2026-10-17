# Security model

A download manager handles untrusted data from servers and, later, from web pages. This page lists what Downpipe trusts, what it does not, and how each boundary is enforced today.

## Untrusted inputs and how they are handled

| Input | Risk | Handling |
|---|---|---|
| `Content-Disposition` and URL file names | Path traversal (`../`), absolute paths, Windows device names (`CON`, `COM1`), reserved characters, hidden extensions via trailing dots | `filename::sanitize`: keeps only the last path component, replaces `<>:"/\|?*` and control characters, strips trailing dots and spaces, prefixes device names, caps length at 200 bytes while keeping the extension. Existing files are never overwritten. |
| Range responses | A server that ignores ranges or changes the file mid-download produces a corrupt file | Every follow-up `206` must match the first offset, total size and ETag; a `200` to a range request is rejected; changes restart the download from zero. |
| Server-sent bytes beyond the announced size | Writing past the end | Writes are clipped to the segment and the announced size. |
| `Retry-After` | A huge value parks a download forever | Capped at 10 minutes. |
| Redirects | Loops | At most 10 redirects. Follow-up requests use the final URL of the current session; resumes go through the original URL again because redirect targets often expire. |
| TLS | Interception | Certificate validation is always on (rustls with the platform verifier). There is no option to turn it off. |

## Secrets

- `user:password@host` in a URL is used for HTTP Basic auth by the HTTP client, but URLs are stored with credentials removed (`redact_url`). Logs use `display_url`, which also drops the query string, because signed URLs carry tokens there.
- Custom request headers (for example `Referer`) are kept in memory only and never written to the meta file or the database. After a restart such downloads continue without them.
- Error messages shown in the UI are built from error classes, not from the HTTP client's display string, which can contain the full URL.
- Downpipe does not read browser cookies. A future opt-in, per-site cookie feature is described in the roadmap and will never log or store cookie values.

## Desktop app

- The web view runs local, bundled content under a strict CSP (`script-src 'self'`, no remote origins).
- It holds only Tauri's `core:default` capability. Folder pickers and "open" / "show in folder" run in Rust, and those commands take a download id, not a path, so a compromised web view still cannot open arbitrary files.
- No network port is opened. The planned browser bridge uses Native Messaging, whose host manifest lists the allowed extension ids explicitly (wildcards are not accepted by browsers).

## Integrity

- Data reaches its final name only through an atomic rename after the last byte is written and, if requested, the checksum matches.
- Resume metadata is written with write-to-temp-then-rename after the data it describes has been synced.

## Planned boundaries

- **Browser extension:** messages from content scripts will be checked in the service worker (`sender.origin`/`sender.url`) and reduced to a fixed schema before reaching the native host; cookies are never part of these messages; host permissions are requested per site rather than `<all_urls>` where possible.
- **External providers (yt-dlp, FFmpeg):** started with argument arrays, never shell strings; their JSON output is validated against a schema before use; DRM-protected media is reported as unsupported.

## Reporting a vulnerability

See [SECURITY.md](../SECURITY.md).
