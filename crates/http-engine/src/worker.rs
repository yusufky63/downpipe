//! One connection streaming one segment into the part file.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use bytes::BytesMut;
use futures_util::StreamExt;
use parking_lot::Mutex;
use reqwest::header::{ETAG, HeaderMap};
use reqwest::{Client, Response, StatusCode};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::status_error;
use crate::http::{self, header_str};
use crate::ratelimit::Limits;
use crate::segments::{Assignment, SegmentId, SegmentMap};
use crate::writer::PartFile;
use crate::{DownloadError, blocking};

/// State shared by the controller and every worker of one download.
#[derive(Debug)]
pub(crate) struct Shared {
    pub map: Mutex<SegmentMap>,
    /// Bytes reserved so far (what the progress bar shows).
    pub downloaded: AtomicU64,
    pub part: Arc<PartFile>,
    /// Connections the host is serving right now (it answered with the
    /// range): how many it has shown it accepts at once.
    pub served: AtomicUsize,
    /// The fastest one connection of this download has been, in bytes per
    /// second over a measuring window: what "far slower" is measured by.
    pub best_rate: AtomicU64,
}

/// How often a connection measures its own speed.
const RATE_WINDOW: Duration = Duration::from_secs(1);
/// A response this old already says how fast a connection can be: fast ones
/// often finish their piece within one `RATE_WINDOW`.
const BEST_AFTER: Duration = Duration::from_millis(250);
/// A connection is never dropped for slowness in its first seconds (TCP slow
/// start, a server warming up).
const SHED_GRACE: Duration = Duration::from_secs(2);
/// "Far slower": below 1/`SHED_RATIO` of the best connection's speed.
const SHED_RATIO: u64 = 4;
/// Only worth a new request if the rest would take longer than this at the
/// connection's own speed.
const SHED_LEFT: Duration = Duration::from_secs(2);

/// Counts a connection in [`Shared::served`] while it lives.
struct Served<'a>(&'a AtomicUsize);

impl<'a> Served<'a> {
    fn new(count: &'a AtomicUsize) -> Self {
        count.fetch_add(1, Ordering::Relaxed);
        Self(count)
    }
}

impl Drop for Served<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Everything a worker needs to issue and validate its request.
#[derive(Debug)]
pub(crate) struct WorkerCtx {
    pub client: Client,
    pub url: Url,
    pub headers: HeaderMap,
    pub total: u64,
    pub etag: Option<String>,
    pub limits: Limits,
    pub buffer_size: usize,
    pub stall_timeout: Duration,
    pub response_timeout: Duration,
    /// The file itself is a web page, so `text/html` answers are expected.
    pub accept_html: bool,
    /// See [`EngineConfig::shed_slow`](crate::EngineConfig::shed_slow).
    pub shed_slow: bool,
}

#[derive(Debug)]
pub(crate) enum WorkerEnd {
    /// The segment is fully written.
    Done,
    /// Asked to stop; whatever arrived is written.
    Stopped,
}

pub(crate) async fn run(
    shared: Arc<Shared>,
    ctx: Arc<WorkerCtx>,
    assignment: Assignment,
    first: Option<Response>,
    stop: CancellationToken,
) -> (SegmentId, Result<WorkerEnd, DownloadError>) {
    let result = stream_segment(&shared, &ctx, assignment, first, &stop).await;
    (assignment.id, result)
}

async fn stream_segment(
    shared: &Shared,
    ctx: &WorkerCtx,
    a: Assignment,
    first: Option<Response>,
    stop: &CancellationToken,
) -> Result<WorkerEnd, DownloadError> {
    let resp = match first {
        Some(resp) => resp,
        None => {
            let req = http::range_get(
                &ctx.client,
                &ctx.url,
                &ctx.headers,
                a.start,
                Some(a.end),
                None,
            );
            // connect_timeout only covers the TCP connect: a server that accepts
            // and then never answers would hold this connection forever.
            let resp = tokio::select! {
                biased;
                () = stop.cancelled() => return Ok(WorkerEnd::Stopped),
                r = tokio::time::timeout(ctx.response_timeout, req.send()) => match r {
                    Err(_) => return Err(DownloadError::Stalled(ctx.response_timeout)),
                    Ok(r) => r.map_err(|e| DownloadError::from_reqwest(&e))?,
                },
            };
            ctx.validate(&resp, a.start)?;
            resp
        }
    };

    let _served = Served::new(&shared.served);
    let mut stream = resp.bytes_stream();
    let mut buf = Buffer::new(ctx.buffer_size);
    let started = Instant::now();
    // Start of the current measuring window and the bytes taken in it, and
    // all bytes taken from this response.
    let mut window = (started, 0u64);
    let mut taken = 0u64;
    let outcome = loop {
        let next = tokio::select! {
            biased;
            () = stop.cancelled() => break Ok(WorkerEnd::Stopped),
            n = tokio::time::timeout(ctx.stall_timeout, stream.next()) => n,
        };
        let chunk = match next {
            Err(_) => break Err(DownloadError::Stalled(ctx.stall_timeout)),
            Ok(None) => break Err(DownloadError::PrematureEnd),
            Ok(Some(Err(e))) => break Err(DownloadError::from_reqwest(&e)),
            Ok(Some(Ok(chunk))) => chunk,
        };
        tokio::select! {
            biased;
            () = stop.cancelled() => break Ok(WorkerEnd::Stopped),
            () = ctx.limits.acquire(chunk.len()) => {}
        }

        let r = shared.map.lock().reserve(a.id, chunk.len() as u64);
        if r.take > 0 {
            buf.push(r.offset, &chunk[..r.take as usize]);
            shared.downloaded.fetch_add(r.take, Ordering::Relaxed);
        }
        if r.exhausted || buf.is_full() {
            buf.flush(shared, a.id).await?;
        }
        if r.exhausted {
            break Ok(WorkerEnd::Done);
        }
        window.1 += r.take;
        taken += r.take;
        let now = Instant::now();
        let age = now.duration_since(started);
        if age >= BEST_AFTER {
            let overall = (taken as f64 / age.as_secs_f64()) as u64;
            shared.best_rate.fetch_max(overall, Ordering::Relaxed);
        }
        let elapsed = now.duration_since(window.0);
        if elapsed >= RATE_WINDOW {
            let rate = (window.1 as f64 / elapsed.as_secs_f64()) as u64;
            window = (now, 0);
            shared.best_rate.fetch_max(rate, Ordering::Relaxed);
            if a.may_shed && ctx.shed_slow && now.duration_since(started) >= SHED_GRACE {
                let best = shared.best_rate.load(Ordering::Relaxed);
                let left = shared.map.lock().unclaimed(a.id);
                if is_far_slower(rate, best, left) && !ctx.limits.limited() {
                    break Err(DownloadError::SlowConnection);
                }
            }
        }
    };
    // Bytes already received are valid whatever ended the loop.
    buf.flush(shared, a.id).await?;
    outcome
}

/// A connection at `rate` bytes/s with `left` bytes to go is worth replacing:
/// far below the best connection's `best`, and with enough left that a new
/// request pays off.
fn is_far_slower(rate: u64, best: u64, left: u64) -> bool {
    rate.saturating_mul(SHED_RATIO) < best && left as f64 > rate as f64 * SHED_LEFT.as_secs_f64()
}

impl WorkerCtx {
    /// A follow-up range response must describe the same file we started
    /// with; anything else would silently corrupt the output.
    fn validate(&self, resp: &Response, start: u64) -> Result<(), DownloadError> {
        // A login or "link expired" page in place of the bytes: retrying
        // will not help, and writing it would corrupt the file.
        if resp.status().is_success() && !self.accept_html && http::is_html(resp) {
            return Err(http::page_error(resp));
        }
        match resp.status() {
            StatusCode::PARTIAL_CONTENT => {
                let cr = http::content_range(resp).ok_or(DownloadError::RangeNotHonoured)?;
                if cr.first != start {
                    return Err(DownloadError::RangeNotHonoured);
                }
                if cr.complete != Some(self.total) {
                    return Err(DownloadError::ResourceChanged("size changed".into()));
                }
                if let (Some(expected), Some(got)) = (&self.etag, header_str(resp.headers(), &ETAG))
                    && *expected != got
                {
                    return Err(DownloadError::ResourceChanged("ETag changed".into()));
                }
                Ok(())
            }
            StatusCode::OK => Err(DownloadError::RangeNotHonoured),
            StatusCode::RANGE_NOT_SATISFIABLE => Err(DownloadError::ResourceChanged(
                "range is no longer satisfiable".into(),
            )),
            status => Err(status_error(status.as_u16(), http::retry_after(resp))),
        }
    }
}

/// Coalesces network chunks into larger positional writes.
#[derive(Debug)]
pub(crate) struct Buffer {
    data: BytesMut,
    offset: u64,
    capacity: usize,
}

impl Buffer {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            data: BytesMut::with_capacity(capacity),
            offset: 0,
            capacity,
        }
    }

    pub(crate) fn push(&mut self, offset: u64, bytes: &[u8]) {
        if self.data.is_empty() {
            self.offset = offset;
        }
        debug_assert_eq!(self.offset + self.data.len() as u64, offset);
        self.data.extend_from_slice(bytes);
    }

    pub(crate) fn is_full(&self) -> bool {
        self.data.len() >= self.capacity
    }

    /// Write to the part file and, for segmented downloads, mark the bytes
    /// as committed in the map.
    pub(crate) async fn flush(
        &mut self,
        shared: &Shared,
        id: SegmentId,
    ) -> Result<(), DownloadError> {
        if self.data.is_empty() {
            return Ok(());
        }
        let end = self.write(&shared.part).await?;
        shared.map.lock().commit(id, end);
        Ok(())
    }

    /// Write and return the end offset of what was written.
    pub(crate) async fn write(&mut self, part: &Arc<PartFile>) -> Result<u64, DownloadError> {
        if self.data.is_empty() {
            return Ok(self.offset);
        }
        let data =
            std::mem::replace(&mut self.data, BytesMut::with_capacity(self.capacity)).freeze();
        let offset = self.offset;
        let end = offset + data.len() as u64;
        let part = part.clone();
        blocking(move || part.write_all_at(offset, &data)).await?;
        self.offset = end;
        Ok(end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_far_slower_connection_with_enough_left_is_replaced() {
        const MIB: u64 = 1024 * 1024;
        // A quarter of the best is not "far" slower yet.
        assert!(!is_far_slower(MIB, 4 * MIB, 10 * MIB));
        assert!(is_far_slower(MIB - 1, 4 * MIB, 10 * MIB));
        // Two seconds or less left at its own pace: not worth a new request.
        assert!(!is_far_slower(64 * 1024, 4 * MIB, 128 * 1024));
        assert!(is_far_slower(64 * 1024, 4 * MIB, 129 * 1024));
        // Nothing to compare with yet.
        assert!(!is_far_slower(0, 0, MIB));
    }
}
