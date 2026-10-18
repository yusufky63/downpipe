//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

mod manager;
pub mod schedule;
mod settings;
mod store;
mod task;
mod torrent;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use settings::{
    Category, MAIN_QUEUE, Queue, Schedule, Settings, TorrentSettings, default_categories,
    default_download_dir, default_queues, extension_of,
};
pub use store::Store;
pub use task::{
    AddOptions, ConfirmOptions, Fragments, Live, MediaChoices, MediaInfo, OnDuplicate, QueueMove,
    SiteChoices, SiteInfo, SiteOptions, Swarm, TaskId, TaskRecord, TaskSnapshot, TaskState,
    TorrentChoices, TorrentFileChoice, TorrentInfo,
};

pub use downpipe_http as http;
pub use downpipe_providers as providers;
pub use torrent::{AVAILABLE as TORRENT_SUPPORT, is_torrent_link};
