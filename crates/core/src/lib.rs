//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

mod manager;
mod settings;
mod store;
mod task;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use settings::{Settings, default_download_dir};
pub use store::Store;
pub use task::{AddOptions, Live, TaskId, TaskRecord, TaskSnapshot, TaskState};

pub use downpipe_http as http;
