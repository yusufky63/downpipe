//! The browser-facing loop of `downpipe-native-host`: read a framed message
//! from stdin, validate it, forward it to the app, write the answer to stdout.

use std::io::{self, Read, Write};
use std::time::Duration;

use crate::frame::{self, FrameError};
use crate::ipc::{self, AppRequest, AppResponse, Endpoint, IpcError};
use crate::protocol::{self, FEATURES, Request, RequestKind, Response, ResponseBody, code};

/// Adding a download is a database write; anything slower means the app hangs.
pub const APP_TIMEOUT: Duration = Duration::from_secs(10);

/// How the host reaches the app. Tests swap in a fake.
pub trait AppLink {
    fn call(&self, request: &AppRequest) -> Result<AppResponse, IpcError>;
}

/// The real link: the local pipe or socket.
#[derive(Debug, Clone)]
pub struct PipeLink {
    pub endpoint: Endpoint,
    pub timeout: Duration,
}

impl AppLink for PipeLink {
    fn call(&self, request: &AppRequest) -> Result<AppResponse, IpcError> {
        ipc::call(&self.endpoint, request, self.timeout)
    }
}

/// Serve messages until the browser closes stdin. A frame over the size limit
/// is answered with an error and ends the session, because the rest of the
/// stream can no longer be trusted to line up with frame boundaries.
pub fn run(
    input: &mut impl Read,
    output: &mut impl Write,
    link: &impl AppLink,
    origin: Option<&str>,
) -> io::Result<()> {
    loop {
        let response = match frame::read_frame(input) {
            Ok(Some(bytes)) => respond(&bytes, link, origin),
            Ok(None) => return Ok(()),
            Err(FrameError::TooLarge(len)) => {
                let message = FrameError::TooLarge(len).to_string();
                send(
                    output,
                    &Response::error(None, code::MESSAGE_TOO_LARGE, message.clone()),
                )?;
                return Err(io::Error::new(io::ErrorKind::InvalidData, message));
            }
            Err(FrameError::Io(e)) => return Err(e),
        };
        send(output, &response)?;
    }
}

fn send(output: &mut impl Write, response: &Response) -> io::Result<()> {
    let payload = serde_json::to_vec(response).map_err(io::Error::other)?;
    match frame::write_frame(output, &payload) {
        Ok(()) => Ok(()),
        Err(FrameError::Io(e)) => Err(e),
        Err(e @ FrameError::TooLarge(_)) => Err(io::Error::other(e.to_string())),
    }
}

/// Answer one message.
pub fn respond(bytes: &[u8], link: &impl AppLink, origin: Option<&str>) -> Response {
    let Request { id, kind } = match protocol::parse_request(bytes) {
        Ok(request) => request,
        Err(rejected) => return Response::rejected(rejected),
    };
    let host_version = env!("CARGO_PKG_VERSION").to_string();
    match kind {
        RequestKind::Ping => {
            let body = match link.call(&AppRequest::ping()) {
                Ok(AppResponse::Pong {
                    app_version,
                    features,
                }) => ResponseBody::Pong {
                    host_version,
                    app_running: true,
                    app_version: Some(app_version),
                    // Only what this host can forward as well.
                    features: features
                        .into_iter()
                        .filter(|f| FEATURES.contains(&f.as_str()))
                        .collect(),
                },
                Err(IpcError::NotRunning) => ResponseBody::Pong {
                    host_version,
                    app_running: false,
                    app_version: None,
                    features: Vec::new(),
                },
                Ok(AppResponse::Error { code, message, .. }) => ResponseBody::Error {
                    code,
                    message,
                    task_id: None,
                },
                Ok(other) => unavailable(&IpcError::BadResponse(format!(
                    "unexpected answer to a ping: {other:?}"
                ))),
                Err(e) => unavailable(&e),
            };
            Response::new(id, body)
        }
        RequestKind::Download(request) => {
            let request = AppRequest::download(request, origin.map(str::to_string));
            Response::new(id, added(link.call(&request), "a download"))
        }
        RequestKind::Media(request) => {
            let request = AppRequest::media(request, origin.map(str::to_string));
            Response::new(id, added(link.call(&request), "a stream"))
        }
        RequestKind::Page(request) => {
            let request = AppRequest::page(request, origin.map(str::to_string));
            Response::new(id, added(link.call(&request), "a page"))
        }
        RequestKind::Tasks(task_ids) => {
            let body = match link.call(&AppRequest::tasks(task_ids)) {
                Ok(AppResponse::Tasks { tasks }) => ResponseBody::Tasks { tasks },
                Ok(AppResponse::Error { code, message, .. }) => ResponseBody::Error {
                    code,
                    message,
                    task_id: None,
                },
                Ok(other) => unavailable(&IpcError::BadResponse(format!(
                    "unexpected answer to a tasks query: {other:?}"
                ))),
                Err(e) => unavailable(&e),
            };
            Response::new(id, body)
        }
    }
}

/// The answer to a `download`, `media` or `page` request: the app took it,
/// or why not.
fn added(answer: Result<AppResponse, IpcError>, what: &str) -> ResponseBody {
    match answer {
        Ok(AppResponse::Accepted {
            task_id,
            file_name,
            awaiting_confirmation,
        }) => ResponseBody::Accepted {
            task_id,
            file_name,
            awaiting_confirmation,
        },
        Ok(AppResponse::Error {
            code,
            message,
            task_id,
        }) => ResponseBody::Error {
            code,
            message,
            task_id,
        },
        Ok(AppResponse::ListOpened { page_url }) => ResponseBody::ListOpened { page_url },
        Ok(other @ (AppResponse::Pong { .. } | AppResponse::Tasks { .. })) => unavailable(
            &IpcError::BadResponse(format!("unexpected answer to {what}: {other:?}")),
        ),
        Err(e) => unavailable(&e),
    }
}

fn unavailable(err: &IpcError) -> ResponseBody {
    let (code, message) = match err {
        IpcError::NotRunning => (
            code::APP_NOT_RUNNING,
            "Downpipe is not running. Open the Downpipe app and try again.".to_string(),
        ),
        other => (
            code::APP_UNAVAILABLE,
            format!("Could not reach Downpipe: {other}"),
        ),
    };
    ResponseBody::Error {
        code: code.to_string(),
        message,
        task_id: None,
    }
}

/// The caller as the browser names it on the command line: Chrome and Edge
/// pass `chrome-extension://<id>/` first; Firefox passes the manifest path and
/// then the add-on id. Only used for logs, and only if it looks like an id.
pub fn origin_from_args(args: &[String]) -> Option<String> {
    let candidate = match args {
        [first, ..] if first.starts_with("chrome-extension://") => first,
        [_, second, ..] => second,
        _ => return None,
    };
    let plausible = candidate.len() <= 128
        && candidate
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@._-{}:/".contains(c));
    plausible.then(|| candidate.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{DownloadRequest, PROTOCOL_VERSION, TaskStatus};
    use serde_json::{Value, json};
    use std::cell::RefCell;

    type Answer = Box<dyn Fn(&AppRequest) -> Result<AppResponse, IpcError>>;

    /// Records what reached the "app" and answers from a script.
    struct FakeApp {
        seen: RefCell<Vec<AppRequest>>,
        answer: Answer,
    }

    impl FakeApp {
        fn new(answer: impl Fn(&AppRequest) -> Result<AppResponse, IpcError> + 'static) -> Self {
            Self {
                seen: RefCell::new(Vec::new()),
                answer: Box::new(answer),
            }
        }

        fn running() -> Self {
            Self::new(|request| {
                Ok(match &request.action {
                    ipc::AppAction::Ping => AppResponse::Pong {
                        app_version: "9.9.9".into(),
                        features: vec!["media".into(), "page".into(), "from-the-future".into()],
                    },
                    ipc::AppAction::Download { request, .. } => AppResponse::Accepted {
                        task_id: "task-1".into(),
                        file_name: request.file_name.clone(),
                        awaiting_confirmation: false,
                    },
                    ipc::AppAction::Media { request, .. } => AppResponse::Accepted {
                        task_id: "stream-1".into(),
                        file_name: request.title.as_ref().map(|t| format!("{t}.mp4")),
                        awaiting_confirmation: true,
                    },
                    ipc::AppAction::Page { request, .. } => AppResponse::Accepted {
                        task_id: "page-1".into(),
                        file_name: request.title.as_ref().map(|t| format!("{t}.mp4")),
                        awaiting_confirmation: true,
                    },
                    ipc::AppAction::Tasks { task_ids } => AppResponse::Tasks {
                        tasks: task_ids
                            .iter()
                            .map(|id| TaskStatus {
                                task_id: id.clone(),
                                state: "downloading".into(),
                                file_name: None,
                                downloaded: 1,
                                total: Some(2),
                                speed: 3,
                                error: None,
                                awaiting_confirmation: false,
                            })
                            .collect(),
                    },
                })
            })
        }
    }

    impl AppLink for FakeApp {
        fn call(&self, request: &AppRequest) -> Result<AppResponse, IpcError> {
            self.seen.borrow_mut().push(request.clone());
            (self.answer)(request)
        }
    }

    fn frames(messages: &[Value]) -> Vec<u8> {
        let mut out = Vec::new();
        for m in messages {
            frame::write_frame(&mut out, m.to_string().as_bytes()).unwrap();
        }
        out
    }

    fn answers(mut output: &[u8]) -> Vec<Value> {
        let mut out = Vec::new();
        while let Some(bytes) = frame::read_frame(&mut output).unwrap() {
            out.push(serde_json::from_slice(&bytes).unwrap());
        }
        out
    }

    #[test]
    fn answers_each_message_in_order_until_eof() {
        let app = FakeApp::running();
        let input = frames(&[
            json!({"protocol_version": 1, "type": "ping", "id": "a"}),
            json!({"protocol_version": 1, "type": "download", "id": "b",
                   "resource_url": "https://example.org/f.zip", "file_name": "f.zip",
                   "referrer": "https://example.org/"}),
            json!({"protocol_version": 1, "type": "download", "id": "c",
                   "resource_url": "javascript:alert(1)"}),
        ]);
        let mut output = Vec::new();
        run(
            &mut input.as_slice(),
            &mut output,
            &app,
            Some("chrome-extension://abc/"),
        )
        .unwrap();
        let out = answers(&output);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["type"], "pong");
        assert_eq!(out[0]["id"], "a");
        assert_eq!(out[0]["app_running"], true);
        assert_eq!(out[0]["app_version"], "9.9.9");
        // Only features this host knows how to forward.
        assert_eq!(out[0]["features"], json!(["media", "page"]));
        assert_eq!(out[1]["type"], "accepted");
        assert_eq!(out[1]["task_id"], "task-1");
        assert_eq!(out[1]["file_name"], "f.zip");
        assert_eq!(out[2]["type"], "error");
        assert_eq!(out[2]["code"], code::INVALID_URL);
        assert_eq!(out[2]["id"], "c");
        for answer in &out {
            assert_eq!(answer["protocol_version"], PROTOCOL_VERSION);
        }

        // The rejected message never reached the app.
        let seen = app.seen.borrow();
        assert_eq!(seen.len(), 2);
        let mut expected = DownloadRequest::new("https://example.org/f.zip");
        expected.file_name = Some("f.zip".into());
        expected.referrer = Some("https://example.org/".into());
        assert_eq!(
            seen[1],
            AppRequest::download(expected, Some("chrome-extension://abc/".into()))
        );
    }

    #[test]
    fn app_not_running_is_a_clear_error_and_a_negative_pong() {
        let app = FakeApp::new(|_| Err(IpcError::NotRunning));
        let ping = respond(br#"{"protocol_version":1,"type":"ping"}"#, &app, None);
        assert!(matches!(
            ping.body,
            ResponseBody::Pong {
                app_running: false,
                app_version: None,
                ..
            }
        ));
        let download = respond(
            br#"{"protocol_version":1,"type":"download","resource_url":"https://example.org/a"}"#,
            &app,
            None,
        );
        match download.body {
            ResponseBody::Error {
                code: c, message, ..
            } => {
                assert_eq!(c, code::APP_NOT_RUNNING);
                assert!(message.contains("not running"), "{message}");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn app_errors_and_timeouts_are_passed_on() {
        let app = FakeApp::new(|_| {
            Ok(AppResponse::Error {
                code: code::DUPLICATE.into(),
                message: "This link is already in Downpipe".into(),
                task_id: Some("t-9".into()),
            })
        });
        let r = respond(
            br#"{"protocol_version":1,"type":"download","resource_url":"https://example.org/a"}"#,
            &app,
            None,
        );
        assert_eq!(
            r.body,
            ResponseBody::Error {
                code: code::DUPLICATE.into(),
                message: "This link is already in Downpipe".into(),
                task_id: Some("t-9".into()),
            }
        );
        let slow = FakeApp::new(|_| Err(IpcError::Timeout(APP_TIMEOUT)));
        let r = respond(br#"{"protocol_version":1,"type":"ping"}"#, &slow, None);
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::APP_UNAVAILABLE)
        );
    }

    #[test]
    fn oversized_frame_gets_an_answer_then_ends_the_session() {
        let app = FakeApp::running();
        let mut input = frames(&[json!({"protocol_version": 1, "type": "ping"})]);
        input.extend_from_slice(&(frame::MAX_MESSAGE_LEN as u32 + 1).to_ne_bytes());
        // A ping after the bad frame must not be processed.
        input.extend(frames(&[json!({"protocol_version": 1, "type": "ping"})]));
        let mut output = Vec::new();
        let result = run(&mut input.as_slice(), &mut output, &app, None);
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::InvalidData);
        let out = answers(&output);
        assert_eq!(out.len(), 2);
        assert_eq!(out[1]["code"], code::MESSAGE_TOO_LARGE);
        assert_eq!(app.seen.borrow().len(), 1);
    }

    #[test]
    fn task_queries_are_forwarded_and_answered() {
        let app = FakeApp::running();
        let r = respond(
            br#"{"protocol_version":1,"type":"tasks","id":"q","task_ids":["a","b"]}"#,
            &app,
            None,
        );
        assert_eq!(r.id.as_deref(), Some("q"));
        match r.body {
            ResponseBody::Tasks { tasks } => {
                assert_eq!(tasks.len(), 2);
                assert_eq!(tasks[1].task_id, "b");
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(
            app.seen.borrow()[0],
            AppRequest::tasks(vec!["a".into(), "b".into()])
        );

        let closed = FakeApp::new(|_| Err(IpcError::NotRunning));
        let r = respond(
            br#"{"protocol_version":1,"type":"tasks","task_ids":["a"]}"#,
            &closed,
            None,
        );
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::APP_NOT_RUNNING)
        );
    }

    #[test]
    fn downloads_held_for_confirmation_are_reported_as_such() {
        let app = FakeApp::new(|request| {
            Ok(match &request.action {
                ipc::AppAction::Download { .. } => AppResponse::Accepted {
                    task_id: "held".into(),
                    file_name: None,
                    awaiting_confirmation: true,
                },
                ipc::AppAction::Tasks { .. } => AppResponse::Tasks {
                    tasks: vec![TaskStatus {
                        task_id: "held".into(),
                        state: "paused".into(),
                        file_name: Some("a.zip".into()),
                        downloaded: 0,
                        total: None,
                        speed: 0,
                        error: None,
                        awaiting_confirmation: true,
                    }],
                },
                ipc::AppAction::Ping
                | ipc::AppAction::Media { .. }
                | ipc::AppAction::Page { .. } => unreachable!(),
            })
        });
        let input = frames(&[
            json!({"protocol_version": 1, "type": "download", "resource_url": "https://example.org/a.zip"}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": ["held"]}),
        ]);
        let mut output = Vec::new();
        run(&mut input.as_slice(), &mut output, &app, None).unwrap();
        let out = answers(&output);
        assert_eq!(out[0]["type"], "accepted");
        assert_eq!(out[0]["awaiting_confirmation"], true);
        assert_eq!(out[1]["tasks"][0]["state"], "paused");
        assert_eq!(out[1]["tasks"][0]["awaiting_confirmation"], true);

        // Not held: the field is left out, as before it existed.
        let plain = FakeApp::running();
        let input = frames(&[
            json!({"protocol_version": 1, "type": "download", "resource_url": "https://example.org/a.zip"}),
            json!({"protocol_version": 1, "type": "tasks", "task_ids": ["task-1"]}),
        ]);
        let mut output = Vec::new();
        run(&mut input.as_slice(), &mut output, &plain, None).unwrap();
        let out = answers(&output);
        assert!(out[0].get("awaiting_confirmation").is_none(), "{}", out[0]);
        assert!(
            out[1]["tasks"][0].get("awaiting_confirmation").is_none(),
            "{}",
            out[1]
        );
    }

    #[test]
    fn streams_are_validated_and_forwarded() {
        let app = FakeApp::running();
        let fixture = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/media-hls.json"),
        )
        .unwrap();
        let r = respond(&fixture, &app, Some("chrome-extension://abc/"));
        assert_eq!(r.id.as_deref(), Some("s1"));
        assert_eq!(
            r.body,
            ResponseBody::Accepted {
                task_id: "stream-1".into(),
                file_name: Some("Clip of the week.mp4".into()),
                awaiting_confirmation: true,
            }
        );
        match &app.seen.borrow()[0].action {
            ipc::AppAction::Media { request, origin } => {
                assert_eq!(
                    request.manifest_url,
                    "https://video.example.net/streams/clip/master.m3u8"
                );
                assert_eq!(request.referrer.as_deref(), Some("https://example.org/"));
                assert_eq!(origin.as_deref(), Some("chrome-extension://abc/"));
            }
            other => panic!("{other:?}"),
        }

        let r = respond(
            br#"{"protocol_version":1,"type":"media","manifest_url":"blob:https://a.org/1","kind":"hls"}"#,
            &app,
            None,
        );
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::INVALID_URL)
        );
        assert_eq!(
            app.seen.borrow().len(),
            1,
            "refused before reaching the app"
        );

        // An app from before streams refuses the action it does not know.
        let old_app = FakeApp::new(|_| {
            Ok(AppResponse::error(
                code::INVALID_MESSAGE,
                "unknown variant `media`",
            ))
        });
        let r = respond(&fixture, &old_app, None);
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::INVALID_MESSAGE)
        );
    }

    #[test]
    fn pages_are_validated_and_forwarded() {
        let app = FakeApp::running();
        let fixture = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/page-youtube.json"),
        )
        .unwrap();
        let r = respond(&fixture, &app, Some("chrome-extension://abc/"));
        assert_eq!(r.id.as_deref(), Some("y1"));
        assert_eq!(
            r.body,
            ResponseBody::Accepted {
                task_id: "page-1".into(),
                file_name: Some("Me at the zoo.mp4".into()),
                awaiting_confirmation: true,
            }
        );
        match &app.seen.borrow()[0].action {
            ipc::AppAction::Page { request, origin } => {
                assert_eq!(
                    request.page_url,
                    "https://www.youtube.com/watch?v=jNQXAC9IVRw"
                );
                assert_eq!(origin.as_deref(), Some("chrome-extension://abc/"));
            }
            other => panic!("{other:?}"),
        }

        // Sites without a provider, and media fields, never reach the app.
        for bad in [
            json!({"protocol_version": 1, "type": "page", "page_url": "https://example.org/watch?v=1"}),
            json!({"protocol_version": 1, "type": "page", "page_url": "https://www.youtube.com/watch?v=jNQXAC9IVRw",
                   "manifest_url": "https://rr1.googlevideo.com/videoplayback"}),
            json!({"protocol_version": 1, "type": "page", "page_url": "https://www.youtube.com/watch?v=jNQXAC9IVRw",
                   "referrer": "https://www.youtube.com/"}),
            json!({"protocol_version": 1, "type": "page"}),
        ] {
            let r = respond(bad.to_string().as_bytes(), &app, None);
            assert!(matches!(r.body, ResponseBody::Error { .. }), "{bad}");
        }
        let r = respond(
            json!({"protocol_version": 1, "type": "page", "page_url": "https://example.org/watch?v=1"})
                .to_string()
                .as_bytes(),
            &app,
            None,
        );
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::UNSUPPORTED_PAGE)
        );
        assert_eq!(
            app.seen.borrow().len(),
            1,
            "refused before reaching the app"
        );
    }

    #[test]
    fn a_pong_from_an_app_without_features_lists_none() {
        let app = FakeApp::new(|_| {
            Ok(AppResponse::Pong {
                app_version: "0.3.0".into(),
                features: Vec::new(),
            })
        });
        let r = respond(br#"{"protocol_version":1,"type":"ping"}"#, &app, None);
        let value = serde_json::to_value(&r).unwrap();
        assert!(value.get("features").is_none(), "{value}");
    }

    #[test]
    fn cookies_never_reach_the_app() {
        let app = FakeApp::running();
        let r = respond(
            br#"{"protocol_version":1,"type":"download","resource_url":"https://example.org/a","cookies":"sid=1"}"#,
            &app,
            None,
        );
        assert!(
            matches!(r.body, ResponseBody::Error { ref code, .. } if code == code::COOKIES_NOT_ACCEPTED)
        );
        assert!(app.seen.borrow().is_empty());
    }

    #[test]
    fn origin_is_read_from_browser_arguments() {
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(
            origin_from_args(&args(&[
                "chrome-extension://abcdef/",
                "--parent-window=123"
            ])),
            Some("chrome-extension://abcdef/".into())
        );
        assert_eq!(
            origin_from_args(&args(&[r"C:\x\host.json", "downpipe@downpipe.dev"])),
            Some("downpipe@downpipe.dev".into())
        );
        assert_eq!(origin_from_args(&args(&[])), None);
        assert_eq!(
            origin_from_args(&args(&["/path/host.json", "evil\nid"])),
            None
        );
    }
}
