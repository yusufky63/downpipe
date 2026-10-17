//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

mod manager;
pub mod schedule;
mod settings;
mod store;
mod task;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use settings::{
    Category, MAIN_QUEUE, Queue, Schedule, Settings, default_categories, default_download_dir,
    default_queues, extension_of,
};
pub use store::Store;
pub use task::{
    AddOptions, ConfirmOptions, Fragments, Live, MediaChoices, MediaInfo, QueueMove, TaskId,
    TaskRecord, TaskSnapshot, TaskState,
};

pub use downpipe_http as http;
