//! One connection streaming one segment into the part file.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bytes::BytesMut;
use futures_util::StreamExt;
use parking_lot::Mutex;
use reqwest::header::{ETAG, HeaderMap};
use reqwest::{Client, Response, StatusCode};
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::status_error;
use crate::http::{self, header_str};
use crate::ratelimit::RateLimiter;
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
}

/// Everything a worker needs to issue and validate its request.
#[derive(Debug)]
pub(crate) struct WorkerCtx {
    pub client: Client,
    pub url: Url,
    pub headers: HeaderMap,
    pub total: u64,
    pub etag: Option<String>,
    pub limiter: Arc<RateLimiter>,
    pub buffer_size: usize,
    pub stall_timeout: Duration,
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
            let resp = tokio::select! {
                biased;
                () = stop.cancelled() => return Ok(WorkerEnd::Stopped),
                r = req.send() => r.map_err(|e| DownloadError::from_reqwest(&e))?,
            };
            ctx.validate(&resp, a.start)?;
            resp
        }
    };

    let mut stream = resp.bytes_stream();
    let mut buf = Buffer::new(ctx.buffer_size);
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
            () = ctx.limiter.acquire(chunk.len()) => {}
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
    };
    // Bytes already received are valid whatever ended the loop.
    buf.flush(shared, a.id).await?;
    outcome
}

impl WorkerCtx {
    /// A follow-up range response must describe the same file we started
    /// with; anything else would silently corrupt the output.
    fn validate(&self, resp: &Response, start: u64) -> Result<(), DownloadError> {
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
