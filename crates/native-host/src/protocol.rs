//! Messages between the browser extension and the native host.
//!
//! Every request is one JSON object with `protocol_version` and `type`:
//!
//! ```json
//! {"protocol_version": 1, "type": "ping"}
//! {"protocol_version": 1, "type": "download", "id": "m1",
//!  "resource_url": "https://example.org/file.zip",
//!  "page_url": "https://example.org/downloads",
//!  "referrer": "https://example.org/",
//!  "mime_type": "application/zip",
//!  "file_name": "file.zip"}
//! {"protocol_version": 1, "type": "tasks", "id": "t1",
//!  "task_ids": ["8e48dd86-5481-41af-a2fc-edf3044e29b8"]}
//! ```
//!
//! Parsing is strict on purpose: unknown or duplicate fields, non-web URLs,
//! control characters and oversized values are rejected instead of ignored,
//! and any field that mentions cookies is refused outright, so an extension
//! bug cannot quietly start shipping session data into the app.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

pub const PROTOCOL_VERSION: u64 = 1;
/// Signed CDN links run to a few kilobytes; nothing legitimate needs more.
pub const MAX_URL_LEN: usize = 8 * 1024;
/// Characters. Most file systems stop at 255.
pub const MAX_FILE_NAME_LEN: usize = 255;
/// RFC 6838 allows 127 characters each for type and subtype.
pub const MAX_MIME_LEN: usize = 255;
pub const MAX_ID_LEN: usize = 64;
/// A `tasks` query asks about the downloads this browser sent recently; the
/// popup lists five.
pub const MAX_TASK_IDS: usize = 20;
/// Error texts in task updates are capped; they end up in a small popup.
pub const MAX_TASK_ERROR_LEN: usize = 300;

/// Stable error codes. The extension branches on these; `message` is for people.
pub mod code {
    pub const INVALID_MESSAGE: &str = "invalid_message";
    pub const UNSUPPORTED_VERSION: &str = "unsupported_version";
    pub const COOKIES_NOT_ACCEPTED: &str = "cookies_not_accepted";
    pub const INVALID_URL: &str = "invalid_url";
    pub const MESSAGE_TOO_LARGE: &str = "message_too_large";
    pub const APP_NOT_RUNNING: &str = "app_not_running";
    pub const APP_UNAVAILABLE: &str = "app_unavailable";
    pub const VERSION_MISMATCH: &str = "version_mismatch";
    pub const DUPLICATE: &str = "duplicate";
    pub const INTERNAL: &str = "internal";
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtocolError {
    #[error("the message is not valid JSON: {0}")]
    Json(String),
    #[error("{0}")]
    Invalid(String),
    #[error(
        "protocol version {0} is not supported; this Downpipe speaks version {PROTOCOL_VERSION}"
    )]
    UnsupportedVersion(String),
    #[error("cookies are never accepted by Downpipe")]
    Cookies,
    #[error("{field}: {reason}")]
    InvalidUrl { field: &'static str, reason: String },
}

impl ProtocolError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Json(_) | Self::Invalid(_) => code::INVALID_MESSAGE,
            Self::UnsupportedVersion(_) => code::UNSUPPORTED_VERSION,
            Self::Cookies => code::COOKIES_NOT_ACCEPTED,
            Self::InvalidUrl { .. } => code::INVALID_URL,
        }
    }
}

/// A request that passed validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// Echoed back in the response so a long-lived port can match answers.
    pub id: Option<String>,
    pub kind: RequestKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestKind {
    /// Is the host installed, and is the app running?
    Ping,
    Download(DownloadRequest),
    /// Progress of downloads the extension added, by the ids `accepted` gave it.
    Tasks(Vec<String>),
}

/// Why a message was refused, plus its `id` when that much could be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rejected {
    pub id: Option<String>,
    pub error: ProtocolError,
}

/// One resource to download. Both ends of the pipe run
/// [`DownloadRequest::validate`]: the host before forwarding and the app
/// before queueing, because anything running as the user can open the pipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DownloadRequest {
    pub resource_url: String,
    /// The page the link was on. Carried for media detection later; the app
    /// does not store it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    /// Sent as `Referer`, which some servers check before serving a file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// A hint only; the app sanitizes it like a server-supplied name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

impl DownloadRequest {
    pub fn new(resource_url: impl Into<String>) -> Self {
        Self {
            resource_url: resource_url.into(),
            page_url: None,
            referrer: None,
            mime_type: None,
            file_name: None,
        }
    }

    /// Check every field and return the normalized request: URLs re-serialized
    /// without fragments, the referrer and page without credentials, the MIME
    /// type reduced to `type/subtype`, and empty optional fields dropped.
    pub fn validate(self) -> Result<Self, ProtocolError> {
        let mut resource = web_url("resource_url", &self.resource_url)?;
        // Never sent to the server; keeping it would only defeat duplicate detection.
        resource.set_fragment(None);
        let page_url = present(self.page_url)
            .map(|raw| web_url("page_url", &raw).map(without_secrets))
            .transpose()?;
        let referrer = present(self.referrer)
            .map(|raw| web_url("referrer", &raw).map(without_secrets))
            .transpose()?;
        let mime_type = present(self.mime_type).map(|m| mime(&m)).transpose()?;
        let file_name = present(self.file_name).map(|n| file_name(&n)).transpose()?;
        Ok(Self {
            resource_url: resource.to_string(),
            page_url: page_url.map(String::from),
            referrer: referrer.map(String::from),
            mime_type,
            file_name,
        })
    }
}

/// Every field any message type may carry. `deny_unknown_fields` turns a typo
/// or an unexpected field (`headers`, `cookies`, ...) into an error.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    #[serde(rename = "protocol_version")]
    _protocol_version: u64,
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "id", default)]
    _id: Option<String>,
    #[serde(default)]
    resource_url: Option<String>,
    #[serde(default)]
    page_url: Option<String>,
    #[serde(default)]
    referrer: Option<String>,
    #[serde(default)]
    mime_type: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    task_ids: Option<Vec<String>>,
}

impl Envelope {
    fn has_download_fields(&self) -> bool {
        [
            &self.resource_url,
            &self.page_url,
            &self.referrer,
            &self.mime_type,
            &self.file_name,
        ]
        .iter()
        .any(|f| f.is_some())
    }
}

/// Parse and validate one message from the extension.
pub fn parse_request(bytes: &[u8]) -> Result<Request, Rejected> {
    let reject = |id: Option<String>, error| Rejected { id, error };
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|e| reject(None, ProtocolError::Json(e.to_string())))?;
    let Some(object) = value.as_object() else {
        return Err(reject(
            None,
            ProtocolError::Invalid("a message must be a JSON object".into()),
        ));
    };

    let id = match object.get("id") {
        None | Some(Value::Null) => None,
        Some(Value::String(id)) if valid_id(id) => Some(id.clone()),
        Some(_) => {
            return Err(reject(
                None,
                ProtocolError::Invalid(format!(
                    "id must be 1 to {MAX_ID_LEN} letters, digits, `-` or `_`"
                )),
            ));
        }
    };
    let fail = |error| reject(id.clone(), error);

    if object
        .keys()
        .any(|k| k.to_ascii_lowercase().contains("cookie"))
    {
        return Err(fail(ProtocolError::Cookies));
    }
    // Checked before the strict parse, so a newer extension gets "unsupported
    // version" rather than a complaint about the fields it added.
    match object.get("protocol_version") {
        Some(Value::Number(n)) if n.as_u64() == Some(PROTOCOL_VERSION) => {}
        Some(Value::Number(n)) => {
            return Err(fail(ProtocolError::UnsupportedVersion(n.to_string())));
        }
        Some(_) => {
            return Err(fail(ProtocolError::Invalid(
                "protocol_version must be a number".into(),
            )));
        }
        None => {
            return Err(fail(ProtocolError::Invalid(
                "protocol_version is missing".into(),
            )));
        }
    }

    // Parse the raw bytes again rather than the `Value`: a `Value` keeps only
    // the last of two identical keys, while the typed parse rejects duplicates.
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|e| fail(ProtocolError::Invalid(e.to_string())))?;
    match envelope.kind.as_str() {
        "ping" => {
            if envelope.has_download_fields() || envelope.task_ids.is_some() {
                return Err(fail(ProtocolError::Invalid(
                    "a ping carries no other fields".into(),
                )));
            }
            Ok(Request {
                id,
                kind: RequestKind::Ping,
            })
        }
        "download" => {
            if envelope.task_ids.is_some() {
                return Err(fail(ProtocolError::Invalid(
                    "task_ids belongs to a `tasks` message".into(),
                )));
            }
            let Some(resource_url) = present(envelope.resource_url) else {
                return Err(fail(ProtocolError::Invalid(
                    "resource_url is required".into(),
                )));
            };
            let request = DownloadRequest {
                resource_url,
                page_url: envelope.page_url,
                referrer: envelope.referrer,
                mime_type: envelope.mime_type,
                file_name: envelope.file_name,
            }
            .validate()
            .map_err(fail)?;
            Ok(Request {
                id,
                kind: RequestKind::Download(request),
            })
        }
        "tasks" => {
            if envelope.has_download_fields() {
                return Err(fail(ProtocolError::Invalid(
                    "a `tasks` message carries only task_ids".into(),
                )));
            }
            let Some(task_ids) = envelope.task_ids else {
                return Err(fail(ProtocolError::Invalid("task_ids is required".into())));
            };
            validate_task_ids(&task_ids).map_err(fail)?;
            Ok(Request {
                id,
                kind: RequestKind::Tasks(task_ids),
            })
        }
        _ => Err(fail(ProtocolError::Invalid(
            "unknown message type; expected `ping`, `download` or `tasks`".into(),
        ))),
    }
}

/// What the host sends back for each request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub protocol_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub body: ResponseBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseBody {
    Pong {
        host_version: String,
        app_running: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        app_version: Option<String>,
    },
    /// Queued in the app.
    Accepted {
        task_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        file_name: Option<String>,
        /// The app holds it until the user answers its confirmation dialog.
        /// See [`TaskStatus::awaiting_confirmation`].
        #[serde(default, skip_serializing_if = "is_false")]
        awaiting_confirmation: bool,
    },
    Error {
        code: String,
        message: String,
        /// For `duplicate`: the download that already exists.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task_id: Option<String>,
    },
    /// Answer to `tasks`. Ids the app does not know (removed downloads) are
    /// left out.
    Tasks { tasks: Vec<TaskStatus> },
}

/// Where one download stands, as the popup shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskStatus {
    pub task_id: String,
    /// `queued`, `starting`, `downloading`, `waiting`, `verifying`, `paused`,
    /// `completed`, `failed` or `canceled`.
    pub state: String,
    /// The final name once the server has answered, else the hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    pub downloaded: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Bytes per second while running.
    pub speed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Sent with `true` only, next to `state: "paused"`, while the app holds
    /// the download for the user to confirm its name, folder and queue.
    ///
    /// A flag rather than a new state: the download really is paused in the
    /// catalog, and extensions from before this field drop entries whose
    /// state they do not know, so a new state would cost them the row's
    /// progress. With the flag they keep showing `paused`, which is true.
    #[serde(default, skip_serializing_if = "is_false")]
    pub awaiting_confirmation: bool,
}

/// Optional flags are left out unless set, so answers stay as they were
/// for extensions that never heard of them.
fn is_false(flag: &bool) -> bool {
    !*flag
}

/// 1 to [`MAX_TASK_IDS`] ids shaped like the ones `accepted` returns. Both
/// ends check this, like [`DownloadRequest::validate`].
pub fn validate_task_ids(ids: &[String]) -> Result<(), ProtocolError> {
    if ids.is_empty() || ids.len() > MAX_TASK_IDS {
        return Err(ProtocolError::Invalid(format!(
            "task_ids must list 1 to {MAX_TASK_IDS} ids"
        )));
    }
    if !ids.iter().all(|id| valid_id(id)) {
        return Err(ProtocolError::Invalid(format!(
            "task ids are 1 to {MAX_ID_LEN} letters, digits, `-` or `_`"
        )));
    }
    Ok(())
}

impl Response {
    pub fn new(id: Option<String>, body: ResponseBody) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id,
            body,
        }
    }

    pub fn error(id: Option<String>, code: &str, message: impl Into<String>) -> Self {
        Self::new(
            id,
            ResponseBody::Error {
                code: code.to_string(),
                message: message.into(),
                task_id: None,
            },
        )
    }

    pub fn rejected(rejected: Rejected) -> Self {
        Self::error(
            rejected.id,
            rejected.error.code(),
            rejected.error.to_string(),
        )
    }
}

fn valid_id(id: &str) -> bool {
    (1..=MAX_ID_LEN).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// `None` for absent and blank fields alike.
fn present(field: Option<String>) -> Option<String> {
    field.filter(|s| !s.trim().is_empty())
}

fn web_url(field: &'static str, raw: &str) -> Result<Url, ProtocolError> {
    let invalid = |reason: String| ProtocolError::InvalidUrl { field, reason };
    if raw.len() > MAX_URL_LEN {
        return Err(invalid(format!("longer than {MAX_URL_LEN} bytes")));
    }
    // The URL parser silently drops tabs and newlines; refuse them instead so
    // what was validated is exactly what was sent.
    if raw.chars().any(char::is_control) {
        return Err(invalid("contains control characters".into()));
    }
    let url = Url::parse(raw.trim()).map_err(|e| invalid(e.to_string()))?;
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(invalid(format!(
                "`{other}:` links cannot be downloaded; only http and https"
            )));
        }
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err(invalid("has no host".into()));
    }
    Ok(url)
}

/// What the Fetch standard strips before using a URL as a referrer.
fn without_secrets(mut url: Url) -> Url {
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    url
}

fn mime(raw: &str) -> Result<String, ProtocolError> {
    if raw.len() > MAX_MIME_LEN {
        return Err(ProtocolError::Invalid(format!(
            "mime_type is longer than {MAX_MIME_LEN} bytes"
        )));
    }
    // Parameters such as `; charset=utf-8` say nothing about the file.
    let essence = raw
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let token = |part: &str| {
        (1..=127).contains(&part.len())
            && part
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"!#$&-^_.+".contains(&b))
    };
    match essence.split_once('/') {
        Some((kind, sub)) if token(kind) && token(sub) => Ok(essence),
        _ => Err(ProtocolError::Invalid(
            "mime_type must look like `application/zip`".into(),
        )),
    }
}

fn file_name(raw: &str) -> Result<String, ProtocolError> {
    let name = raw.trim();
    if name.chars().count() > MAX_FILE_NAME_LEN {
        return Err(ProtocolError::Invalid(format!(
            "file_name is longer than {MAX_FILE_NAME_LEN} characters"
        )));
    }
    if name.chars().any(char::is_control) {
        return Err(ProtocolError::Invalid(
            "file_name contains control characters".into(),
        ));
    }
    if name.contains(['/', '\\']) || name == "." || name == ".." {
        return Err(ProtocolError::Invalid(
            "file_name must be a bare file name without folders".into(),
        ));
    }
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(value: Value) -> Result<Request, Rejected> {
        parse_request(value.to_string().as_bytes())
    }

    fn download(value: Value) -> Result<DownloadRequest, ProtocolError> {
        let mut message = json!({"protocol_version": 1, "type": "download"});
        message
            .as_object_mut()
            .unwrap()
            .extend(value.as_object().unwrap().clone());
        match parse(message) {
            Ok(Request {
                kind: RequestKind::Download(d),
                ..
            }) => Ok(d),
            Ok(other) => panic!("not a download: {other:?}"),
            Err(r) => Err(r.error),
        }
    }

    fn code_of(result: Result<DownloadRequest, ProtocolError>) -> &'static str {
        result.expect_err("should be rejected").code()
    }

    #[test]
    fn parses_ping_with_and_without_id() {
        let ping = parse(json!({"protocol_version": 1, "type": "ping"})).unwrap();
        assert_eq!(ping.kind, RequestKind::Ping);
        assert_eq!(ping.id, None);
        let ping = parse(json!({"protocol_version": 1, "type": "ping", "id": "p-1"})).unwrap();
        assert_eq!(ping.id.as_deref(), Some("p-1"));
    }

    #[test]
    fn parses_and_normalizes_a_full_download() {
        let d = download(json!({
            "resource_url": " https://cdn.example.org/a/file.zip?sig=abc#part ",
            "page_url": "https://user:pw@example.org/downloads#list",
            "referrer": "https://example.org/downloads#list",
            "mime_type": "Application/ZIP; charset=binary",
            "file_name": "  file.zip ",
        }))
        .unwrap();
        assert_eq!(d.resource_url, "https://cdn.example.org/a/file.zip?sig=abc");
        assert_eq!(d.page_url.as_deref(), Some("https://example.org/downloads"));
        assert_eq!(d.referrer.as_deref(), Some("https://example.org/downloads"));
        assert_eq!(d.mime_type.as_deref(), Some("application/zip"));
        assert_eq!(d.file_name.as_deref(), Some("file.zip"));
    }

    #[test]
    fn blank_optional_fields_are_dropped() {
        let d = download(json!({
            "resource_url": "http://example.org/x",
            "page_url": "",
            "referrer": "  ",
            "mime_type": null,
            "file_name": "",
        }))
        .unwrap();
        assert_eq!(d, DownloadRequest::new("http://example.org/x"));
    }

    #[test]
    fn only_http_and_https_resources_are_accepted() {
        for url in [
            "javascript:alert(1)",
            "data:text/plain,hi",
            "file:///C:/Windows/win.ini",
            "blob:https://example.org/0f1e",
            "ftp://example.org/file",
            "chrome://settings",
            "not a url",
            "",
        ] {
            let result = download(json!({ "resource_url": url }));
            let expected = if url.is_empty() {
                code::INVALID_MESSAGE
            } else {
                code::INVALID_URL
            };
            assert_eq!(code_of(result), expected, "{url}");
        }
        assert_eq!(
            code_of(download(
                json!({"resource_url": "https://x.org/", "referrer": "file:///etc/passwd"})
            )),
            code::INVALID_URL
        );
    }

    #[test]
    fn control_characters_are_rejected_not_stripped() {
        assert_eq!(
            code_of(download(
                json!({"resource_url": "https://example.org/a\nb"})
            )),
            code::INVALID_URL
        );
        assert_eq!(
            code_of(download(json!({
                "resource_url": "https://example.org/",
                "referrer": "https://example.org/\r\nCookie: a=b",
            }))),
            code::INVALID_URL
        );
        assert_eq!(
            code_of(download(json!({
                "resource_url": "https://example.org/",
                "file_name": "a\u{0}.exe",
            }))),
            code::INVALID_MESSAGE
        );
    }

    #[test]
    fn sizes_are_limited() {
        let long_url = format!("https://example.org/{}", "a".repeat(MAX_URL_LEN));
        assert_eq!(
            code_of(download(json!({ "resource_url": long_url }))),
            code::INVALID_URL
        );
        let name = "n".repeat(MAX_FILE_NAME_LEN + 1);
        assert_eq!(
            code_of(download(
                json!({"resource_url": "https://example.org/", "file_name": name})
            )),
            code::INVALID_MESSAGE
        );
        let ok_name = "\u{e9}".repeat(MAX_FILE_NAME_LEN);
        assert!(
            download(json!({"resource_url": "https://example.org/", "file_name": ok_name})).is_ok()
        );
        let mime = format!("application/{}", "x".repeat(300));
        assert_eq!(
            code_of(download(
                json!({"resource_url": "https://example.org/", "mime_type": mime})
            )),
            code::INVALID_MESSAGE
        );
        let id = "i".repeat(MAX_ID_LEN + 1);
        let r = parse(json!({"protocol_version": 1, "type": "ping", "id": id})).unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        assert_eq!(r.id, None);
    }

    #[test]
    fn file_names_must_not_carry_folders() {
        for name in ["../evil.exe", "a/b.zip", r"C:\Windows\x.dll", "..", "."] {
            assert_eq!(
                code_of(download(
                    json!({"resource_url": "https://example.org/", "file_name": name})
                )),
                code::INVALID_MESSAGE,
                "{name}"
            );
        }
    }

    #[test]
    fn mime_types_must_be_type_slash_subtype() {
        for mime in ["zip", "application/", "/zip", "text/html extra", "a/b/c"] {
            assert_eq!(
                code_of(download(
                    json!({"resource_url": "https://example.org/", "mime_type": mime})
                )),
                code::INVALID_MESSAGE,
                "{mime}"
            );
        }
        let d = download(json!({
            "resource_url": "https://example.org/",
            "mime_type": "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        }))
        .unwrap();
        assert!(d.mime_type.unwrap().starts_with("application/vnd."));
    }

    #[test]
    fn cookies_are_refused_in_any_spelling() {
        for key in [
            "cookies",
            "cookie",
            "Cookie",
            "set_cookie",
            "requestCookies",
        ] {
            let mut message = json!({
                "protocol_version": 1,
                "type": "download",
                "id": "c1",
                "resource_url": "https://example.org/file",
            });
            message[key] = json!("session=secret");
            let r = parse(message).unwrap_err();
            assert_eq!(r.error, ProtocolError::Cookies, "{key}");
            assert_eq!(r.id.as_deref(), Some("c1"));
        }
    }

    #[test]
    fn unknown_and_duplicate_fields_are_rejected() {
        let r = parse(json!({
            "protocol_version": 1,
            "type": "download",
            "resource_url": "https://example.org/",
            "headers": {"Authorization": "Bearer x"},
        }))
        .unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        assert!(r.error.to_string().contains("headers"));

        let dup = br#"{"protocol_version":1,"type":"download","resource_url":"https://a.org/","resource_url":"https://b.org/"}"#;
        let r = parse_request(dup).unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        assert!(r.error.to_string().contains("duplicate"));
    }

    #[test]
    fn versions_and_types_are_checked() {
        let r = parse(json!({"protocol_version": 2, "type": "ping", "new_field": 1})).unwrap_err();
        assert_eq!(r.error.code(), code::UNSUPPORTED_VERSION);
        for (version, expected) in [
            (json!("1"), code::INVALID_MESSAGE),
            (json!(null), code::INVALID_MESSAGE),
            (json!(1.5), code::UNSUPPORTED_VERSION),
        ] {
            let r = parse(json!({"protocol_version": version, "type": "ping"})).unwrap_err();
            assert_eq!(r.error.code(), expected, "{version}");
        }
        let r = parse(json!({"type": "ping"})).unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        let r = parse(json!({"protocol_version": 1, "type": "delete_everything"})).unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        let r =
            parse(json!({"protocol_version": 1, "type": "ping", "resource_url": "https://a.org/"}))
                .unwrap_err();
        assert_eq!(r.error.code(), code::INVALID_MESSAGE);
    }

    #[test]
    fn parses_task_queries() {
        let id = "8e48dd86-5481-41af-a2fc-edf3044e29b8";
        let r =
            parse(json!({"protocol_version": 1, "type": "tasks", "id": "t1", "task_ids": [id]}))
                .unwrap();
        assert_eq!(r.kind, RequestKind::Tasks(vec![id.to_string()]));
        assert_eq!(r.id.as_deref(), Some("t1"));
    }

    #[test]
    fn task_queries_are_bounded_and_kept_apart() {
        let too_many: Vec<String> = (0..=MAX_TASK_IDS).map(|i| format!("t{i}")).collect();
        for bad in [
            json!({"protocol_version": 1, "type": "tasks", "task_ids": []}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": too_many}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": ["../etc"]}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": [1]}),
            json!({"protocol_version": 1, "type": "tasks"}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": ["a"], "resource_url": "https://a.org/"}),
            json!({"protocol_version": 1, "type": "download", "resource_url": "https://a.org/", "task_ids": ["a"]}),
            json!({"protocol_version": 1, "type": "ping", "task_ids": ["a"]}),
        ] {
            let r = parse(bad.clone()).unwrap_err();
            assert_eq!(r.error.code(), code::INVALID_MESSAGE, "{bad}");
        }
    }

    #[test]
    fn task_updates_serialize_without_empty_fields() {
        let running = TaskStatus {
            task_id: "t".into(),
            state: "downloading".into(),
            file_name: Some("a.zip".into()),
            downloaded: 10,
            total: None,
            speed: 5,
            error: None,
            awaiting_confirmation: false,
        };
        let held = TaskStatus {
            task_id: "u".into(),
            state: "paused".into(),
            file_name: None,
            downloaded: 0,
            total: None,
            speed: 0,
            error: None,
            awaiting_confirmation: true,
        };
        let body = ResponseBody::Tasks {
            tasks: vec![running, held],
        };
        let response = Response::new(Some("q".into()), body);
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(
            value,
            json!({"protocol_version": 1, "id": "q", "type": "tasks", "tasks": [
                {"task_id": "t", "state": "downloading", "file_name": "a.zip", "downloaded": 10, "speed": 5},
                {"task_id": "u", "state": "paused", "downloaded": 0, "speed": 0, "awaiting_confirmation": true}
            ]})
        );
        assert_eq!(serde_json::from_value::<Response>(value).unwrap(), response);
    }

    #[test]
    fn malformed_json_and_non_objects_are_rejected() {
        for raw in [
            &b"not json"[..],
            b"[1,2]",
            b"\"ping\"",
            b"",
            b"{\"protocol_version\":1,",
        ] {
            let r = parse_request(raw).unwrap_err();
            assert_eq!(r.error.code(), code::INVALID_MESSAGE);
        }
    }

    #[test]
    fn responses_serialize_flat_with_type_tags() {
        let pong = Response::new(
            Some("p1".into()),
            ResponseBody::Pong {
                host_version: "0.2.0".into(),
                app_running: false,
                app_version: None,
            },
        );
        assert_eq!(
            serde_json::to_value(&pong).unwrap(),
            json!({"protocol_version": 1, "id": "p1", "type": "pong", "host_version": "0.2.0", "app_running": false})
        );
        let accepted = Response::new(
            None,
            ResponseBody::Accepted {
                task_id: "t".into(),
                file_name: Some("a.zip".into()),
                awaiting_confirmation: false,
            },
        );
        assert_eq!(
            serde_json::to_value(&accepted).unwrap(),
            json!({"protocol_version": 1, "type": "accepted", "task_id": "t", "file_name": "a.zip"})
        );
        let held = Response::new(
            None,
            ResponseBody::Accepted {
                task_id: "t".into(),
                file_name: None,
                awaiting_confirmation: true,
            },
        );
        assert_eq!(
            serde_json::to_value(&held).unwrap(),
            json!({"protocol_version": 1, "type": "accepted", "task_id": "t", "awaiting_confirmation": true})
        );
        let error = Response::error(None, code::APP_NOT_RUNNING, "Downpipe is not running");
        let text = serde_json::to_string(&error).unwrap();
        assert_eq!(serde_json::from_str::<Response>(&text).unwrap(), error);
    }

    /// The same files are read by the extension's tests, so both sides agree
    /// on what a valid message looks like.
    #[test]
    fn shared_fixtures_parse() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let ping = std::fs::read(dir.join("ping.json")).unwrap();
        assert_eq!(parse_request(&ping).unwrap().kind, RequestKind::Ping);
        let link = std::fs::read(dir.join("download-link.json")).unwrap();
        let Request {
            kind: RequestKind::Download(d),
            id,
        } = parse_request(&link).unwrap()
        else {
            panic!("not a download");
        };
        assert_eq!(id.as_deref(), Some("m1"));
        assert_eq!(
            d.clone().validate().unwrap(),
            d,
            "fixture is already normalized"
        );
        let captured = std::fs::read(dir.join("download-captured.json")).unwrap();
        assert!(matches!(
            parse_request(&captured).unwrap().kind,
            RequestKind::Download(_)
        ));
        let tasks = std::fs::read(dir.join("tasks.json")).unwrap();
        assert!(matches!(
            parse_request(&tasks).unwrap().kind,
            RequestKind::Tasks(ids) if ids.len() == 2
        ));
        let cookies = std::fs::read(dir.join("rejected-cookies.json")).unwrap();
        assert_eq!(
            parse_request(&cookies).unwrap_err().error,
            ProtocolError::Cookies
        );
    }
}
