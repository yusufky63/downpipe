//! Downpipe's browser bridge, both ends of it.
//!
//! ```text
//! extension --Native Messaging (stdio)--> downpipe-native-host
//!           --named pipe / Unix socket--> desktop app --> Manager::add
//! ```
//!
//! * [`protocol`]: browser-facing messages and their validation.
//! * [`frame`]: the length-prefixed framing used on both legs.
//! * [`ipc`]: the local endpoint and the host's client for it.
//! * [`server`]: the desktop app's end, feeding [`downpipe_core::Manager`].
//! * [`host`]: the host's message loop.
//! * [`launch`]: starting the app next to the host when it is closed.
//! * [`manifest`]: host manifests and per-browser registration.

pub mod frame;
pub mod host;
pub mod ipc;
pub mod launch;
pub mod manifest;
pub mod protocol;
pub mod server;
