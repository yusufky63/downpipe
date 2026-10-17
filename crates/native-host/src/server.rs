//! The desktop app's end of the bridge: serves the local [`Endpoint`] and
//! turns validated requests into [`Manager::add`] calls.

use std::io;
use std::sync::Arc;
use std::time::Duration;

use downpipe_core::{AddOptions, CoreError, Manager, TaskSnapshot};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::frame::{self, FrameError};
use crate::ipc::{AppAction, AppRequest, AppResponse, Endpoint};
use crate::protocol::{self, DownloadRequest, MAX_TASK_ERROR_LEN, TaskStatus, code};

/// A client that connects and then says nothing is dropped after this long.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
pub struct BridgeServer {
    endpoint: Endpoint,
    #[cfg(windows)]
    pipe: tokio::net::windows::named_pipe::NamedPipeServer,
    #[cfg(unix)]
    listener: tokio::net::UnixListener,
    #[cfg(unix)]
    _socket: SocketFile,
}

impl BridgeServer {
    /// Claim the endpoint. Fails if another process already serves it, for
    /// example a second Downpipe window. Must run inside a Tokio runtime.
    pub fn bind(endpoint: Endpoint) -> io::Result<Self> {
        #[cfg(windows)]
        {
            // `first_pipe_instance` makes this fail instead of silently
            // joining a pipe someone else created under the same name.
            let pipe = create_pipe(&endpoint, true).map_err(|e| {
                if e.kind() == io::ErrorKind::PermissionDenied {
                    io::Error::new(
                        io::ErrorKind::AddrInUse,
                        format!("{endpoint} is already served by another process"),
                    )
                } else {
                    e
                }
            })?;
            Ok(Self { endpoint, pipe })
        }
        #[cfg(unix)]
        {
            let listener = bind_socket(endpoint.path())?;
            let socket = SocketFile(endpoint.path().to_owned());
            Ok(Self {
                endpoint,
                listener,
                _socket: socket,
            })
        }
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Answer requests until the future is dropped.
    pub async fn serve(self, manager: Manager, app_version: impl Into<String>) {
        let bridge = Arc::new(Bridge {
            manager,
            app_version: app_version.into(),
        });
        self.accept_loop(bridge).await;
    }

    #[cfg(windows)]
    async fn accept_loop(self, bridge: Arc<Bridge>) {
        let Self { endpoint, pipe, .. } = self;
        let mut waiting = pipe;
        loop {
            let connected = waiting.connect().await;
            // Create the next instance before handing this one off, so the
            // name is never left without an instance another process could claim.
            let next = match create_pipe(&endpoint, false) {
                Ok(next) => next,
                Err(e) => {
                    tracing::warn!("browser bridge stopped: {e}");
                    return;
                }
            };
            let current = std::mem::replace(&mut waiting, next);
            match connected {
                Ok(()) => {
                    tokio::spawn(handle(current, bridge.clone()));
                }
                Err(e) => tracing::debug!("bridge client left before it was served: {e}"),
            }
        }
    }

    #[cfg(unix)]
    async fn accept_loop(self, bridge: Arc<Bridge>) {
        loop {
            match self.listener.accept().await {
                Ok((stream, _)) => {
                    tokio::spawn(handle(stream, bridge.clone()));
                }
                Err(e) => {
                    tracing::warn!("browser bridge accept failed: {e}");
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }
}

#[cfg(windows)]
fn create_pipe(
    endpoint: &Endpoint,
    first: bool,
) -> io::Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    // The default security descriptor gives write access only to the
    // creating user, SYSTEM and administrators, and its medium integrity
    // label keeps sandboxed (low integrity) processes out.
    tokio::net::windows::named_pipe::ServerOptions::new()
        .first_pipe_instance(first)
        .reject_remote_clients(true)
        .create(endpoint.path())
}

#[cfg(unix)]
fn bind_socket(path: &std::path::Path) -> io::Result<tokio::net::UnixListener> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "the socket path has no folder")
    })?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)?;
    // Never loosen or tighten a folder we did not create; just refuse one
    // that other users can enter.
    if std::fs::metadata(dir)?.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} is open to other users; the browser bridge needs a private folder",
                dir.display()
            ),
        ));
    }
    if std::fs::symlink_metadata(path).is_ok() {
        if std::os::unix::net::UnixStream::connect(path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("{} is already served by another process", path.display()),
            ));
        }
        // Left behind by an app that did not exit cleanly.
        std::fs::remove_file(path)?;
    }
    let listener = tokio::net::UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Removes the socket file when the server goes away.
#[cfg(unix)]
#[derive(Debug)]
struct SocketFile(std::path::PathBuf);

#[cfg(unix)]
impl Drop for SocketFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[derive(Debug)]
struct Bridge {
    manager: Manager,
    app_version: String,
}

impl Bridge {
    async fn dispatch(&self, bytes: &[u8]) -> AppResponse {
        let request = match AppRequest::parse(bytes) {
            Ok(request) => request,
            Err(answer) => return answer,
        };
        match request.action {
            AppAction::Ping => AppResponse::Pong {
                app_version: self.app_version.clone(),
            },
            AppAction::Download { request, origin } => {
                // Anything running as this user can open the pipe, so the
                // host's validation is not taken on trust.
                let request = match request.validate() {
                    Ok(request) => request,
                    Err(e) => return AppResponse::error(e.code(), e.to_string()),
                };
                let manager = self.manager.clone();
                // `Manager::add` writes to SQLite.
                tokio::task::spawn_blocking(move || add(&manager, request, origin.as_deref()))
                    .await
                    .unwrap_or_else(|e| AppResponse::error(code::INTERNAL, e.to_string()))
            }
            AppAction::Tasks { task_ids } => {
                if let Err(e) = protocol::validate_task_ids(&task_ids) {
                    return AppResponse::error(e.code(), e.to_string());
                }
                // Only the ids asked for: the extension learns about the
                // downloads it sent, not the rest of the list.
                let tasks = task_ids
                    .iter()
                    .filter_map(|id| self.manager.get(id))
                    .map(task_status)
                    .collect();
                AppResponse::Tasks { tasks }
            }
        }
    }
}

fn task_status(task: TaskSnapshot) -> TaskStatus {
    let record = task.record;
    let file_name = record
        .path
        .as_deref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().into_owned())
        .or(record.file_name);
    TaskStatus {
        task_id: record.id,
        state: record.state.as_str().to_string(),
        file_name,
        downloaded: record.downloaded,
        total: record.total,
        speed: task.live.speed,
        error: record
            .error
            .map(|e| e.chars().take(MAX_TASK_ERROR_LEN).collect()),
    }
}

fn add(manager: &Manager, request: DownloadRequest, origin: Option<&str>) -> AppResponse {
    if let Ok(url) = url::Url::parse(&request.resource_url) {
        tracing::info!(
            origin = origin.unwrap_or("unknown"),
            url = %downpipe_core::http::display_url(&url),
            "download from the browser"
        );
    }
    let headers = request
        .referrer
        .map(|referrer| vec![("Referer".to_string(), referrer)])
        .unwrap_or_default();
    let options = AddOptions {
        url: request.resource_url,
        file_name: request.file_name,
        headers,
        ..AddOptions::default()
    };
    match manager.add(options) {
        Ok(task) => AppResponse::Accepted {
            task_id: task.record.id,
            file_name: task.record.file_name,
        },
        Err(CoreError::Duplicate(id)) => AppResponse::Error {
            code: code::DUPLICATE.to_string(),
            message: "This link is already in Downpipe".into(),
            task_id: Some(id),
        },
        Err(e) => AppResponse::error(e.code(), e.to_string()),
    }
}

async fn handle<S>(mut stream: S, bridge: Arc<Bridge>)
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let response =
        match tokio::time::timeout(IO_TIMEOUT, frame::read_frame_async(&mut stream)).await {
            Ok(Ok(Some(bytes))) => bridge.dispatch(&bytes).await,
            Ok(Ok(None)) => return,
            Ok(Err(e @ FrameError::TooLarge(_))) => {
                AppResponse::error(code::MESSAGE_TOO_LARGE, e.to_string())
            }
            Ok(Err(FrameError::Io(e))) => {
                tracing::debug!("bridge read failed: {e}");
                return;
            }
            Err(_) => {
                tracing::debug!("bridge client sent nothing in time");
                return;
            }
        };
    let Ok(payload) = serde_json::to_vec(&response) else {
        return;
    };
    if let Ok(Ok(())) =
        tokio::time::timeout(IO_TIMEOUT, frame::write_frame_async(&mut stream, &payload)).await
    {
        // Let the client hang up first, so the answer can never be thrown
        // away with the pipe before it was read.
        let mut byte = [0u8; 1];
        let _ = tokio::time::timeout(IO_TIMEOUT, stream.read(&mut byte)).await;
    }
}
