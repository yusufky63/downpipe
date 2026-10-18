//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

mod manager;
pub mod post;
pub mod schedule;
mod settings;
mod store;
mod task;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use post::{
    CommandReport, CommandResult, ExtractReport, ExtractResult, PostActions, PostReport,
    PostSettings, PostStep, ScanReport, ScanResult,
};
pub use settings::{
    Category, MAIN_QUEUE, Queue, Schedule, Settings, default_categories, default_download_dir,
    default_queues, extension_of,
};
pub use store::Store;
pub use task::{
    AddOptions, ConfirmOptions, Fragments, Live, MediaChoices, MediaInfo, OnDuplicate, QueueMove,
    SiteChoices, SiteInfo, SiteOptions, TaskId, TaskRecord, TaskSnapshot, TaskState,
};

pub use downpipe_http as http;
pub use downpipe_providers as providers;
