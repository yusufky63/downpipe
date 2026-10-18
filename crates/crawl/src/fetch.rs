//! Reading a page with a size cap, a time limit and a way to stop.

use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use reqwest::{Client, Method};
use tokio_util::sync::CancellationToken;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FetchError {
    #[error("stopped")]
    Canceled,
    #[error("the server did not answer in time")]
    Timeout,
    #[error("{0}")]
    Http(#[from] reqwest::Error),
}

#[derive(Debug)]
pub(crate) struct Page {
    /// Where redirects ended.
    pub url: Url,
    pub status: u16,
    pub content_type: Option<String>,
    pub body: String,
    /// The body was longer than the cap and was cut.
    pub truncated: bool,
}

impl Page {
    pub fn is_html(&self) -> bool {
        self.content_type.as_deref().is_some_and(|t| {
            let essence = t
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase();
            essence == "text/html" || essence == "application/xhtml+xml"
        })
    }
}

/// GET `url` and read at most `max` bytes of its body as text.
pub(crate) async fn get_text(
    client: &Client,
    url: &Url,
    accept: &str,
    max: usize,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Page, FetchError> {
    let work = async {
        let resp = client
            .get(url.clone())
            .header(ACCEPT, accept)
            .send()
            .await?;
        let final_url = resp.url().clone();
        let status = resp.status().as_u16();
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let mut body = Vec::new();
        let mut truncated = false;
        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let room = max - body.len();
            if chunk.len() > room {
                body.extend_from_slice(&chunk[..room]);
                truncated = true;
                break;
            }
            body.extend_from_slice(&chunk);
        }
        Ok::<_, FetchError>(Page {
            url: final_url,
            status,
            content_type,
            body: String::from_utf8_lossy(&body).into_owned(),
            truncated,
        })
    };
    tokio::select! {
        _ = cancel.cancelled() => Err(FetchError::Canceled),
        r = tokio::time::timeout(timeout, work) => r.map_err(|_| FetchError::Timeout)?,
    }
}

/// The size a `HEAD` request reports, if the server answers one.
pub(crate) async fn head_size(
    client: &Client,
    url: &Url,
    timeout: Duration,
    cancel: &CancellationToken,
) -> Result<Option<u64>, FetchError> {
    let work = async {
        let resp = client.request(Method::HEAD, url.clone()).send().await?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        // reqwest reports 0 for HEAD answers; read the header itself.
        Ok::<_, FetchError>(
            resp.headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.trim().parse::<u64>().ok()),
        )
    };
    tokio::select! {
        _ = cancel.cancelled() => Err(FetchError::Canceled),
        r = tokio::time::timeout(timeout, work) => r.map_err(|_| FetchError::Timeout)?,
    }
}

/// Wait `delay`, unless stopped first.
pub(crate) async fn pause(delay: Duration, cancel: &CancellationToken) -> Result<(), FetchError> {
    if delay.is_zero() {
        return Ok(());
    }
    tokio::select! {
        _ = cancel.cancelled() => Err(FetchError::Canceled),
        _ = tokio::time::sleep(delay) => Ok(()),
    }
}
