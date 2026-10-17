# Changelog

All notable changes to Downpipe. Dates are YYYY-MM-DD.

## 0.1.0 — unreleased

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
