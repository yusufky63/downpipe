# Downpipe

An open-source download manager for Windows, macOS and Linux. Downpipe splits a file across several HTTP connections when that makes it faster, survives pauses, restarts and crashes without starting over, and never publishes a file under its real name until every byte is in place.

It is built as a Rust engine with a desktop app (Tauri) and a command-line tool on top. Browser integration and web-media support are planned in that order; see [Roadmap](#roadmap).

> Status: early development (0.1). The HTTP engine, queue, CLI and desktop app work and are tested; there are no signed releases yet.

## What works today

| Area | Details |
|---|---|
| Multi-connection HTTP | Byte-range requests over several HTTP/1.1 connections, each with its own TCP congestion window. |
| Adaptive connections | Starts with 2 connections and doubles only while measured throughput improves by at least 10 %, up to 8 per download by default (configurable, max 32). |
| Dynamic segmentation | A connection that finishes takes over half of the largest remaining segment, so a slow segment never holds up the end of a download. |
| Resume | Progress is checkpointed next to the file (`name.part.meta`). Resumes are validated with `If-Range` plus ETag and size checks; if the remote file changed, the download restarts cleanly instead of mixing two versions. |
| Crash safety | Data is synced to disk before the checkpoint that claims it is written. The final file appears only after an atomic rename. |
| Failure handling | Retries with exponential backoff and jitter, honours `Retry-After`, reduces connections when the server rate limits (429/503), and tells auth, 403, 404 and network errors apart. |
| Integrity | Optional SHA-256 / SHA-512 checksum, verified before the file is renamed into place. |
| Queue | Persistent queue (SQLite) with a concurrency limit, pause/resume/cancel, duplicate detection, and automatic continuation after the app restarts. |
| Limits | Global speed limit shared by all downloads, adjustable while they run. |
| Safety | Server-supplied file names are sanitized (path traversal, Windows device names, reserved characters); existing files are never overwritten. |
| Desktop app | Download list with per-segment progress, details panel, add-links dialog (paste several at once), settings, light and dark themes. |
| CLI | `downpipe get`, `downpipe resume`, `downpipe probe`. Ctrl+C pauses cleanly. |

What Downpipe does **not** do: it does not bypass DRM, paywalls or logins, it does not read browser cookies, and it does not send your downloads or links to any server of ours. See [Legal](docs/LEGAL.md).

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

## Command line

```text
downpipe get <URL> [-o DIR] [-n NAME] [-c MAX_CONNECTIONS] [--limit RATE] [--checksum sha256:HEX] [-H "Name: value"] [--proxy URL]
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

Exit codes: `0` finished, `1` failed, `2` paused or canceled.

## How the engine works

1. **Open.** The first request is a real `GET` with `Range: bytes=0-`. A `206 Partial Content` answer proves range support and that connection keeps streaming as the first worker, so no request is spent on a separate `HEAD`. A `200` answer means one connection.
2. **Split.** When another connection is allowed, it takes the second half of the segment with the most bytes left. The original connection keeps reading and stops at the new boundary.
3. **Measure.** Every two seconds of saturated transfer the engine compares throughput with the previous connection count and doubles only when it helped.
4. **Checkpoint.** Every five seconds, and always on pause, written data is synced and the finished ranges are recorded in `name.part.meta` with the ETag, Last-Modified and size they belong to.
5. **Finish.** After the last byte, the optional checksum is verified and `name.part` is renamed to `name` (or `name (1)` if that exists by then).

Details: [docs/DOWNLOAD_ENGINE.md](docs/DOWNLOAD_ENGINE.md). Component overview: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Security model: [docs/SECURITY_MODEL.md](docs/SECURITY_MODEL.md).

## Repository layout

```text
crates/http-engine   downpipe-http   probe, segments, workers, resume meta, checksum, rate limit
crates/core          downpipe-core   queue, state machine, SQLite catalog, settings
crates/cli           downpipe        command-line interface
apps/desktop         Tauri 2 + React desktop app (src-tauri is the Rust side)
tools/test-server    fault-injection HTTP server used by tests and for benchmarks
docs/                architecture, engine, security and legal notes
```

## Development

All checks:

```powershell
cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test; pnpm check
```

Tests run against `tools/test-server`, a local server that can ignore ranges, drop connections, return 429/503 with `Retry-After`, change the file between requests, throttle one region of a file and more. Nothing in the test suite touches the internet. To try the app or CLI against it:

```powershell
cargo run -p downpipe-test-server
```

It prints ready-to-use URLs for each failure mode.

## Roadmap

The order follows one rule: each step has to work end to end before the next one starts.

1. **HTTP engine, resume, adaptive segmentation, queue, CLI, desktop shell** — done (0.1).
2. **Browser bridge.** Chrome/Edge/Firefox extension that hands downloads to the app through Native Messaging (no local HTTP port), with origin checks and no cookies in messages.
3. **Media detection.** Find the real manifest or media request behind a page instead of `blob:` URLs; HLS and DASH downloads with fragment concurrency.
4. **Site providers.** Optional, separately updated adapter around yt-dlp for supported sites, a format chooser, and FFmpeg for muxing separate audio and video. Downloads of DRM-protected media are refused.
5. **Local API and web UI**, then a plugin SDK.

## Legal

Downpipe is intended for downloading files and media that you own, have permission to download, that are made downloadable by the relevant service, or that may otherwise be lawfully downloaded. The project does not provide DRM circumvention and does not guarantee that use with a supported website is permitted by that website's terms.

Downpipe; sahibi olduğunuz, indirme izniniz bulunan, ilgili hizmetin indirilebilir olarak sunduğu ya da başka bir şekilde yasal olarak indirilebilen dosya ve medyaları indirmek için tasarlanmıştır. Proje DRM korumasını aşma imkânı sunmaz ve desteklenen bir sitede kullanımın o sitenin kullanım koşullarınca serbest olduğunu garanti etmez.

More in [docs/LEGAL.md](docs/LEGAL.md).

## License

Apache License 2.0. See [LICENSE](LICENSE) and [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
