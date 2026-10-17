//! The local pipe between `downpipe-native-host` and the running desktop app.
//!
//! * Windows: a named pipe, `\\.\pipe\downpipe-bridge-<user>`, that refuses
//!   remote clients. The host connects with identification-only impersonation
//!   rights, so whoever serves the pipe cannot act with the user's token.
//! * Elsewhere: a Unix domain socket in a folder only the user can open.
//!
//! Neither opens a network port. Each connection carries exactly one request
//! and one response, framed like Native Messaging (see [`crate::frame`]).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::frame::{self, FrameError};
use crate::protocol::{DownloadRequest, MediaRequest, TaskStatus, code};

/// Bumped whenever host and app stop understanding each other. They ship
/// together, but a user can update one without re-registering the other.
pub const IPC_VERSION: u32 = 1;
/// Overrides the endpoint for tests and portable setups. Both the host and
/// the app read it.
pub const ENDPOINT_ENV: &str = "DOWNPIPE_BRIDGE_ENDPOINT";

/// Where the desktop app listens: a pipe name on Windows, a socket path elsewhere.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint(PathBuf);

impl Endpoint {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// [`ENDPOINT_ENV`] if set, otherwise [`Endpoint::default_for_user`].
    pub fn from_env_or_default() -> Self {
        match std::env::var_os(ENDPOINT_ENV) {
            Some(path) if !path.is_empty() => Self::new(path),
            _ => Self::default_for_user(),
        }
    }

    #[cfg(windows)]
    pub fn default_for_user() -> Self {
        // Pipe names are machine-wide, so the user name keeps two people on
        // one machine from colliding.
        let user: String = std::env::var("USERNAME")
            .unwrap_or_default()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .take(64)
            .collect::<String>()
            .to_ascii_lowercase();
        let user = if user.is_empty() { "user".into() } else { user };
        Self::new(format!(r"\\.\pipe\downpipe-bridge-{user}"))
    }

    #[cfg(unix)]
    pub fn default_for_user() -> Self {
        // XDG_RUNTIME_DIR is private to the user on Linux; macOS has none, and
        // its Application Support folder sits inside the user's home.
        let base = dirs::runtime_dir()
            .or_else(dirs::data_local_dir)
            .unwrap_or_else(std::env::temp_dir);
        Self::new(base.join("downpipe").join("bridge.sock"))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.display().fmt(f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppRequest {
    pub ipc_version: u32,
    pub action: AppAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum AppAction {
    Ping,
    Download {
        request: DownloadRequest,
        /// The caller as the browser reported it to the host
        /// (`chrome-extension://<id>/` or a Firefox add-on id). For logs.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
    },
    /// A stream for the app to read and download. Added after the first
    /// release of this pipe; an app from before it answers `invalid_message`.
    Media {
        request: MediaRequest,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        origin: Option<String>,
    },
    Tasks {
        task_ids: Vec<String>,
    },
}

impl AppRequest {
    pub fn ping() -> Self {
        Self {
            ipc_version: IPC_VERSION,
            action: AppAction::Ping,
        }
    }

    pub fn download(request: DownloadRequest, origin: Option<String>) -> Self {
        Self {
            ipc_version: IPC_VERSION,
            action: AppAction::Download { request, origin },
        }
    }

    pub fn media(request: MediaRequest, origin: Option<String>) -> Self {
        Self {
            ipc_version: IPC_VERSION,
            action: AppAction::Media { request, origin },
        }
    }

    pub fn tasks(task_ids: Vec<String>) -> Self {
        Self {
            ipc_version: IPC_VERSION,
            action: AppAction::Tasks { task_ids },
        }
    }

    /// Parse a request on the app side. The version is checked first, so a
    /// newer host gets a clear answer instead of a complaint about new fields.
    pub fn parse(bytes: &[u8]) -> Result<Self, AppResponse> {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|e| AppResponse::error(code::INVALID_MESSAGE, e.to_string()))?;
        match value.get("ipc_version").and_then(Value::as_u64) {
            Some(v) if v == u64::from(IPC_VERSION) => {}
            _ => {
                return Err(AppResponse::error(
                    code::VERSION_MISMATCH,
                    "Downpipe and its browser bridge are different versions. \
                     Reinstall the browser bridge from this version of Downpipe.",
                ));
            }
        }
        serde_json::from_slice(bytes)
            .map_err(|e| AppResponse::error(code::INVALID_MESSAGE, e.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppResponse {
    Pong {
        app_version: String,
        /// What this app supports beyond the first messages (see
        /// [`crate::protocol::FEATURES`]). Missing from apps before it.
        #[serde(default)]
        features: Vec<String>,
    },
    Accepted {
        task_id: String,
        #[serde(default)]
        file_name: Option<String>,
        /// Defaulted, so a host and an app one version apart still agree:
        /// neither side rejects fields it does not know here.
        #[serde(default)]
        awaiting_confirmation: bool,
    },
    Error {
        code: String,
        message: String,
        #[serde(default)]
        task_id: Option<String>,
    },
    Tasks {
        tasks: Vec<TaskStatus>,
    },
}

impl AppResponse {
    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::Error {
            code: code.to_string(),
            message: message.into(),
            task_id: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IpcError {
    #[error("Downpipe is not running")]
    NotRunning,
    #[error("Downpipe did not answer within {} seconds", .0.as_secs())]
    Timeout(Duration),
    #[error("Downpipe sent an answer this browser bridge does not understand: {0}")]
    BadResponse(String),
    #[error(transparent)]
    Io(#[from] io::Error),
}

impl From<FrameError> for IpcError {
    fn from(err: FrameError) -> Self {
        match err {
            FrameError::TooLarge(_) => Self::BadResponse(err.to_string()),
            FrameError::Io(e) => Self::Io(e),
        }
    }
}

/// Send one request to the app and wait at most `timeout` for the answer.
pub fn call(
    endpoint: &Endpoint,
    request: &AppRequest,
    timeout: Duration,
) -> Result<AppResponse, IpcError> {
    let payload = serde_json::to_vec(request).map_err(io::Error::other)?;
    let endpoint = endpoint.clone();
    let deadline = Instant::now() + timeout;
    // Blocking pipe reads have no timeout of their own on Windows, so the
    // round trip runs on a thread that is simply abandoned if the app hangs.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(round_trip(&endpoint, &payload, deadline));
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(_) => Err(IpcError::Timeout(timeout)),
    }
}

fn round_trip(
    endpoint: &Endpoint,
    payload: &[u8],
    deadline: Instant,
) -> Result<AppResponse, IpcError> {
    let mut stream = connect(endpoint, deadline)?;
    frame::write_frame(&mut stream, payload)?;
    let answer = frame::read_frame(&mut stream)?
        .ok_or_else(|| IpcError::BadResponse("the connection closed without an answer".into()))?;
    serde_json::from_slice(&answer).map_err(|e| IpcError::BadResponse(e.to_string()))
}

#[cfg(windows)]
fn connect(endpoint: &Endpoint, deadline: Instant) -> Result<std::fs::File, IpcError> {
    use std::os::windows::fs::OpenOptionsExt;

    const ERROR_PIPE_BUSY: i32 = 231;
    // SECURITY_IDENTIFICATION: the server may learn who connected but cannot
    // impersonate the user. Matters if another account squats the pipe name.
    const SECURITY_IDENTIFICATION: u32 = 0x0001_0000;

    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .security_qos_flags(SECURITY_IDENTIFICATION)
            .open(endpoint.path())
        {
            Ok(file) => return Ok(file),
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Err(IpcError::NotRunning),
            // Every instance is serving someone; the app creates a new one
            // right after accepting, so this clears within milliseconds.
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(e.into()),
        }
    }
}

#[cfg(unix)]
fn connect(
    endpoint: &Endpoint,
    _deadline: Instant,
) -> Result<std::os::unix::net::UnixStream, IpcError> {
    match std::os::unix::net::UnixStream::connect(endpoint.path()) {
        Ok(stream) => Ok(stream),
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            Err(IpcError::NotRunning)
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn requests_round_trip_through_json() {
        let mut d = DownloadRequest::new("https://example.org/a.zip");
        d.referrer = Some("https://example.org/".into());
        let mut m = MediaRequest::new(
            "https://video.example.org/master.m3u8",
            downpipe_core::http::media::MediaKind::Hls,
        );
        m.title = Some("Clip".into());
        for request in [
            AppRequest::ping(),
            AppRequest::download(d, Some("chrome-extension://abc/".into())),
            AppRequest::media(m, None),
            AppRequest::tasks(vec!["t-1".into(), "t-2".into()]),
        ] {
            let bytes = serde_json::to_vec(&request).unwrap();
            assert_eq!(AppRequest::parse(&bytes).unwrap(), request);
        }
        assert_eq!(
            serde_json::to_value(AppRequest::ping()).unwrap(),
            json!({"ipc_version": IPC_VERSION, "action": "ping"})
        );
    }

    #[test]
    fn version_is_checked_before_fields() {
        let newer = json!({"ipc_version": IPC_VERSION + 1, "action": "ping", "extra": true});
        match AppRequest::parse(newer.to_string().as_bytes()) {
            Err(AppResponse::Error { code: c, .. }) => assert_eq!(c, code::VERSION_MISMATCH),
            other => panic!("{other:?}"),
        }
        let unknown = json!({"ipc_version": IPC_VERSION, "action": "ping", "extra": true});
        match AppRequest::parse(unknown.to_string().as_bytes()) {
            Err(AppResponse::Error { code: c, .. }) => assert_eq!(c, code::INVALID_MESSAGE),
            other => panic!("{other:?}"),
        }
        let cookie = json!({
            "ipc_version": IPC_VERSION,
            "action": {"download": {"request": {"resource_url": "https://a.org/", "cookies": "x"}}},
        });
        assert!(AppRequest::parse(cookie.to_string().as_bytes()).is_err());
    }

    /// Host and app can be one update apart; the confirmation flag must not
    /// make either side refuse the other's answer.
    #[test]
    fn answers_without_or_with_newer_fields_still_parse() {
        let older = json!({"type": "accepted", "task_id": "t", "file_name": null});
        assert_eq!(
            serde_json::from_value::<AppResponse>(older).unwrap(),
            AppResponse::Accepted {
                task_id: "t".into(),
                file_name: None,
                awaiting_confirmation: false,
            }
        );
        let older_pong = json!({"type": "pong", "app_version": "0.3.0"});
        assert_eq!(
            serde_json::from_value::<AppResponse>(older_pong).unwrap(),
            AppResponse::Pong {
                app_version: "0.3.0".into(),
                features: Vec::new(),
            }
        );
        let older_tasks = json!({"type": "tasks", "tasks": [
            {"task_id": "t", "state": "paused", "downloaded": 0, "speed": 0}
        ]});
        match serde_json::from_value::<AppResponse>(older_tasks).unwrap() {
            AppResponse::Tasks { tasks } => assert!(!tasks[0].awaiting_confirmation),
            other => panic!("{other:?}"),
        }
        let newer = json!({"type": "tasks", "tasks": [
            {"task_id": "t", "state": "paused", "downloaded": 0, "speed": 0,
             "awaiting_confirmation": true, "added_later": 1}
        ]});
        match serde_json::from_value::<AppResponse>(newer).unwrap() {
            AppResponse::Tasks { tasks } => assert!(tasks[0].awaiting_confirmation),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn missing_endpoint_means_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let endpoint = if cfg!(windows) {
            Endpoint::new(format!(r"\\.\pipe\downpipe-test-{}", uuid::Uuid::new_v4()))
        } else {
            Endpoint::new(dir.path().join("none.sock"))
        };
        let result = call(&endpoint, &AppRequest::ping(), Duration::from_secs(2));
        assert!(matches!(result, Err(IpcError::NotRunning)), "{result:?}");
    }

    #[test]
    fn default_endpoint_is_per_user() {
        let endpoint = Endpoint::default_for_user();
        let text = endpoint.to_string();
        if cfg!(windows) {
            assert!(text.starts_with(r"\\.\pipe\downpipe-bridge-"), "{text}");
        } else {
            assert!(text.ends_with("bridge.sock"), "{text}");
        }
    }
}
