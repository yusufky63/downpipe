# Download engine

How `downpipe-http` turns a URL into a complete, verified file. Default values below come from `EngineConfig::default()` in `crates/http-engine/src/config.rs`.

## Defaults

| Setting | Default | Meaning |
|---|---|---|
| `max_connections` | 8 | Upper bound of connections per download (1 to 32). |
| `initial_connections` | 2 | Connections before the first measurement. |
| `min_split_size` | 1 MiB | A segment is only split if both halves would be at least this big. |
| `buffer_size` | 256 KiB | Bytes buffered per connection before a positional write. |
| `ramp_window` | 2 s | Saturated-transfer time measured before deciding to add connections. |
| `ramp_gain` | 1.10 | A doubling must raise throughput by 10 % to be kept. |
| `checkpoint_interval` | 5 s | How often data is synced and the meta file updated. |
| `stall_timeout` | 30 s | A connection that delivers nothing for this long is restarted. |
| `response_timeout` | 60 s | How long a request may wait for the response headers before it is sent again. |
| `max_retries` | 8 | Consecutive server failures per segment before giving up. Progress resets the count. |
| `offline_grace` | 10 min | How long network failures are retried while no byte arrives at all. |
| `retry_base_delay` / `retry_max_delay` | 0.5 s / 30 s | Exponential backoff with ±20 % jitter. |

## Opening a download

The engine sends `GET` with `Range: bytes=0-` and `Accept-Encoding: identity` (byte offsets must refer to the stored representation, not a compressed variant).

| Response | Meaning | What happens |
|---|---|---|
| `206` with `Content-Range: bytes 0-N/TOTAL` | Ranges work, size known | Segmented and resumable. This response becomes the first worker. |
| `200` | Range ignored | One connection, start to end. Not resumable. |
| `416` | Usually an empty file | Asked again without `Range`. |
| `401`, `407` | Login needed | Fails with `auth_required`. |
| `403` | Refused | Fails with `forbidden` (expired link, missing Referer or cookies). |
| `404`, `410` | Gone | Fails with `not_found`. |
| `429`, `503` | Rate limited | Waits for `Retry-After` (capped at 10 minutes) or backoff, then retries. |
| other `5xx`, `408`, network errors, no headers within `response_timeout` | Transient | Retried with backoff (up to 5 attempts for the first request of a new download). |
| `200`/`206` with `text/html` for a file named like a binary (`.zip`, `.exe`, `.pdf`, `.mp4`, ...) | A sign-in or "link expired" page | Fails with `login_required` when the URL it ended at looks like a sign-in page (`/login`, `accounts.`, `sso`, ...), else `expired_link`. Nothing is saved. |

### File names

The name comes from `Content-Disposition`, then the URLs, then `download`, and is sanitized and made unique in the target folder (`name (1).ext`).

- `Content-Disposition` follows RFC 6266 with the leniency browsers show: `filename*` (RFC 5987, UTF-8 or ISO-8859-1, quoted or not, `filename*0*=` continuations) wins over `filename` in any order. A plain `filename` may be raw UTF-8 (read from the header's bytes, with ISO-8859-1 as the fallback), percent-encoded UTF-8, or an RFC 2047 word (`=?UTF-8?B?...?=`). The disposition type may be missing and a closing quote forgotten. A broken `filename*` falls back to `filename`.
- From URLs, the last path segment, percent-decoded (kept encoded if it is not UTF-8). After a redirect to a storage URL named by a hash, the link's own name wins when only it has an extension.
- An extension is added from the MIME type when the name has none (a version number such as `v1.2` is not one).
- Sanitizing replaces `< > : " / \ | ? *` and control characters with `_`, removes invisible direction and zero-width marks (U+202E makes `exe.pdf` read as `fdp.exe`), trims trailing dots and spaces and leading spaces, prefixes Windows device names (`CON`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`, also with `¹²³` and an extension) with `_`, and cuts the name to 200 bytes keeping the extension. A table of real-world headers and a seeded random test hold `filename.rs` to this.

A single-connection download that fails deletes its partial file: without resume data it is useless, and left behind it would push the retry to `name (1)`.

Data is written to `name.part`. On NTFS the part file is marked sparse first: otherwise writing at the end of a large file makes Windows zero-fill everything before it synchronously.

## Segments and work stealing

A `SegmentMap` tracks byte ranges with three cursors each: `start <= written <= reserved <= end`.

- A worker *reserves* bytes under the map lock before buffering them, so a split can never hand the same bytes to two connections.
- *Written* advances after the bytes reach the file; that is what checkpoints record.
- When a connection is free it takes an idle segment if there is one. Otherwise it splits the active segment with the most unreserved bytes at its midpoint (aligned to 64 KiB) and takes the second half. The victim keeps reading its original response and stops at the new end.

Because the split always goes to the largest remaining segment, the last seconds of a download are shared by all connections instead of waiting for one slow one.

Every follow-up `206` must report the same total size and, if the server sends one, the same ETag. A mismatch means the file changed: the download restarts once from zero rather than combining two versions.

## How many connections

All connections run HTTP/1.1. Under HTTP/2 every segment would share one TCP connection and one congestion window, which removes the benefit.

1. Start with `initial_connections`.
2. Once every allowed connection is busy, measure throughput for `ramp_window`.
3. Double the connection count, measure again.
4. If the new rate is less than `ramp_gain` times the previous one, go back to the previous count and stop probing.
5. A `429` or `503` reduces the count to the connections still running and stops probing for this download.

Many hosts cap connections per client and refuse one too many with `403`, `429` or `503` while the others are served. A refusal of that kind, while other connections of the same download are running, is not a failure: the count drops to the connections that work, the segment goes to the next free one, and probing stops. The engine remembers the working count per host (`host:port`) for as long as it runs, so later downloads from that host start within it instead of tripping the limit again (`Engine::connection_limit`, and `Engine::connection_limits` for all of them). Engines made with `Engine::derive` (another proxy or user agent) share that memory. A `403` with no other connection left is retried like a server failure once the host has shown such a limit (it may still count a connection that just closed); a link that really expired runs out of retries and fails with `forbidden`.

## Retries, silence and network outages

A segment that fails is handed back and retried after a backoff. How long it keeps trying depends on the failure:

- **Server failures** (`5xx`, `429` with no other connection, a range the server stops honouring) count against `max_retries`, per segment and in a row. Bytes written reset the count; re-arming a segment after its backoff does not.
- **Network failures** (offline, DNS, connection refused or reset, a response cut short, silence) are retried until no byte has arrived from any connection for `offline_grace`, however many attempts that takes. A dropped Wi-Fi, a switch between networks or a VPN reconnect therefore does not fail a download: it waits, and continues by itself once requests get through again. As soon as one connection delivers bytes, segments still sitting out a long backoff from the outage are retried at once.
- **Silence.** A connection that delivers nothing for `stall_timeout` is closed and its segment retried; a request that gets no response headers within `response_timeout` is sent again. Without these, a server that accepts a connection and then says nothing would hold it forever.

The first request of a new download gives up after 5 attempts, so a mistyped or dead host is reported quickly. A resume, or the restart of a single-connection download, waits out network failures like a running download: the host served the file before.

A running download never shows as downloading while no bytes move. `Progress::phase` becomes `Waiting`, and `Progress::waiting` says why: `network` (requests fail before reaching the server), `no_data` (connections are open but silent for 10 s, or half the stall timeout if shorter), `throttled` (`429`, `503`), or `server`. The desktop app shows it under the download's name.

## Resume

`name.part.meta` (JSON) holds the request URL (credentials removed), the size, ETag, Last-Modified, MIME type, optional checksum and the merged list of finished ranges:

```json
{
  "version": 1,
  "url": "https://example.org/file.iso",
  "final_url": "https://cdn.example.org/file.iso",
  "file_name": "file.iso",
  "total_size": 8589934592,
  "etag": "\"18a273bd-200000000\"",
  "completed": [{ "start": 0, "end": 536870912 }],
  "created_at": 1792181400,
  "updated_at": 1792181460
}
```

Checkpoints sync the part file *before* writing the meta (write to `.tmp`, then rename), so the meta never claims bytes a power cut could lose. They run beside the transfer loop, so a slow disk flush does not stall connection management.

Starting the same URL into the same folder finds the meta and sends the first missing range with `If-Range` set to the strong ETag (weak ETags are not allowed there, so Last-Modified is used instead). The resume is accepted only if the server answers `206` for exactly that offset and total size with the same ETag. Anything else (`200`, `416`, a different size or ETag, a missing or truncated part file) discards the old data and starts over, with a notice explaining why.

A web page (`text/html`) in answer to a resume of a file that is not one is checked first: it is a sign-in or "link expired" page, not a new version of the file, so the download fails with `login_required` or `expired_link` and the partial file and its meta stay as they were. The same goes for a web page arriving on any later connection or fragment request. The desktop app offers **Use a new link…** for these failures (and for `forbidden`) in place of **Retry**, which would get the same page.

A download from a server without range support cannot resume; pausing it deletes the partial file. Pausing while a resume is still waiting for the server's first answer keeps the partial file and its meta untouched.

## Replacing an expired link

`DownloadRequest::relink` tells the engine that the URL replaces the one the resume data was saved with (the desktop app's **Use a new link**). The meta is then found by file name instead of URL, and the resume goes through the same checks as any other: `If-Range` with the saved validator, then the `206` must report the saved size and ETag. If it does, the meta is rewritten for the new link at once, so later resumes are ordinary. If it does not, the download fails with `different_file` and nothing on disk is touched; a normal resume would have started over instead.

## Finishing

1. Final checkpoint and sync.
2. If a checksum was given, the part file is hashed (SHA-256 or SHA-512, streamed). On mismatch the download fails with `checksum` and the part file is kept for inspection; the real file name is never used.
3. `name.part` is renamed to `name`; if a file with that name appeared meanwhile, `name (1)` and so on.
4. The meta file is deleted.

## Streams (HLS and DASH)

A request with `media` set downloads the stream its URL describes instead of the file itself.

1. **Manifest.** Fetched with the request's headers (at most 8 MiB, retried like the first request of a file). HLS: a master playlist lists qualities and audio renditions; a media playlist lists segments. DASH: one static MPD lists everything. Protected streams (HLS `SAMPLE-AES`, key formats other than plain AES-128, session keys; DASH `ContentProtection`) and live streams (HLS without `#EXT-X-ENDLIST` or `PLAYLIST-TYPE:VOD`, DASH `type="dynamic"`) are refused here, before any media request.
2. **Plan.** The chosen quality, or the best one (height, then bandwidth), and its audio: the chosen one if it fits, else one in the same language, else the default. A quality that carries its own sound is one track; otherwise video and audio are two. HLS fetches the quality's and the audio's media playlists. A stored choice is found again by id, then by height and the closest bandwidth.
3. **Name.** The request's name (or the URL's) gets the extension of the result: `.mp4` for fragmented MP4 or merged tracks, `.ts` for transport streams without FFmpeg, `.webm`, `.mkv` for mixed containers, `.m4a` or `.aac` for audio only.
4. **Fragments.** The init section and every segment of every track, `max_connections` at a time. Each is written to `t<track>-<index>.tmp` in the work folder `<name>.part/`, synced, decrypted if it is AES-128 (the key is fetched once per URI and kept in memory), and renamed to `.frag`. A byte-range segment asks for its range; a partial `.tmp` continues with a range request. Failures are retried with the same backoff as file segments, and `429`/`503` halves the number of fragments in flight.
5. **Join.** Each track's fragments are concatenated into `track<n>.<ext>` and synced; only then are the fragments deleted and `media.json` updated.
6. **Merge.** With FFmpeg: `ffmpeg -i video -i audio -map 0:v:0 -map 1:a:0 -c copy out.mp4` (no re-encoding; `-bsf:a aac_adtstoasc` for AAC from transport streams), or the same with one input to turn a transport stream into MP4. Without FFmpeg, or if it fails, the video is saved as `name.<ext>` and the audio as `name.audio.<ext>`, with a notice. The result is renamed into place and the work folder removed.

Progress counts finished fragments and the bytes of those in flight. The total is an estimate (from the bandwidth and duration, then from the average finished fragment) until the last fragment is in. The segment bar shows the fragments as equal slices.

**Resume.** `media.json` in the work folder records the manifest URL, the chosen tracks and, per track, a fingerprint of its fragment list (paths, ranges and encryption, not query strings, which carry tokens that change on every fetch). A resume fetches the manifest again; if it lists the same fragments, existing `.frag` files are kept and only the rest is downloaded, otherwise the folder is emptied and the stream starts over. Pausing while FFmpeg runs stops it; the joined tracks stay, and the next start only merges.

## Speed limit

One token bucket per engine (the desktop app and CLI use one engine). Connections take tokens for each chunk and sleep off any debt, which keeps the total rate exact even with Windows' coarse timer. The rate can change while downloads run.

A download can have a bucket of its own as well (`DownloadRequest::limiter`). Its connections take from that one first and then from the engine's, so the download runs at the lower of the two limits while the global one still holds for all of them together. Taking from its own bucket first means a download sleeping off its own debt does not hold tokens other downloads could use. The owner of the bucket (the core) can change its rate while the download runs.
`DownloadRequest::site_limiter` adds a third bucket, which the desktop app shares among the downloads one site rule applies to. A connection takes from the download's own bucket, then the site's, then the engine's, so the lowest of the three limits wins.

## Per-download overrides

Besides `max_connections`, `limiter` and `site_limiter`, a request can set `max_retries` (in place of `EngineConfig::max_retries`, also for the first request and for streams). A proxy is a property of the client, so a download through another proxy runs on an engine from `Engine::derive`, which shares the limiter and the host memory; `EngineConfig::no_proxy` connects directly, ignoring the system proxy. The desktop app's site rules use all of these ([site-rules.md](site-rules.md)).

## Measuring

`tools/test-server` can cap the rate per connection (`rate=`), slow down part of a file (`slow_from=`, `slow_rate=`), drop connections (`drop_after=`, `drop_times=`), fail requests (`fail=`, `fail_times=`, `fail_from=`, `retry_after=`), refuse more than N bodies at once (`max_conns=`, `limit_status=`), go silent mid-body (`stall_at=`, `stall_times=`) or before the headers (`hang_times=`), answer with a "link expired" page (`html=1`, `html_after=`) or redirect to a sign-in page (`/redirect?to=/login`), and send a `Content-Disposition` exactly as given (`cd=`). `TestServer::stop` and `restart` take it off the network and back, killing open connections too, as a Wi-Fi switch would. The tests in `crates/http-engine/tests/robustness.rs` use these. Example, with a per-connection cap of 8 MiB/s, where the engine ramps 2 -> 4 -> 8 connections:

```powershell
cargo run --release -p downpipe-test-server
```

```powershell
$env:DOWNPIPE_LOG="downpipe_http=debug"; cargo run --release -p downpipe-cli -- get "http://127.0.0.1:17655/file/402653184?rate=8388608" -o $env:TEMP
```
