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
| `max_retries` | 8 | Consecutive failures per segment before giving up. Progress resets the count. |
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
| other `5xx`, `408`, network errors | Transient | Retried with backoff (up to 5 attempts for the first request). |

The file name comes from `Content-Disposition` (`filename*` preferred over `filename`), then the last URL path segment, then `download`; an extension is added from the MIME type when the name has none. The name is sanitized and made unique in the target folder.

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

A download from a server without range support cannot resume; pausing it deletes the partial file.

## Finishing

1. Final checkpoint and sync.
2. If a checksum was given, the part file is hashed (SHA-256 or SHA-512, streamed). On mismatch the download fails with `checksum` and the part file is kept for inspection; the real file name is never used.
3. `name.part` is renamed to `name`; if a file with that name appeared meanwhile, `name (1)` and so on.
4. The meta file is deleted.

## Speed limit

One token bucket per engine (the desktop app and CLI use one engine). Connections take tokens for each chunk and sleep off any debt, which keeps the total rate exact even with Windows' coarse timer. The rate can change while downloads run.

## Measuring

`tools/test-server` can cap the rate per connection (`rate=`), slow down part of a file (`slow_from=`, `slow_rate=`), drop connections (`drop_after=`, `drop_times=`) or fail requests (`fail=`, `fail_times=`, `retry_after=`). Example, with a per-connection cap of 8 MiB/s, where the engine ramps 2 -> 4 -> 8 connections:

```powershell
cargo run --release -p downpipe-test-server
```

```powershell
$env:DOWNPIPE_LOG="downpipe_http=debug"; cargo run --release -p downpipe-cli -- get "http://127.0.0.1:17655/file/402653184?rate=8388608" -o $env:TEMP
```
