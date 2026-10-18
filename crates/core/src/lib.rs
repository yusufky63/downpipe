//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

pub mod backup;
pub mod feeds;
pub mod idm;
mod manager;
pub mod post;
pub mod rules;
pub mod schedule;
mod settings;
mod store;
mod task;
mod torrent;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use post::{
    CommandReport, CommandResult, ExtractReport, ExtractResult, PostActions, PostReport,
    PostSettings, PostStep, ScanReport, ScanResult,
};
pub use rules::{RefusedHost, RuleProblem, RuleProxy, RuleTest, SiteRule};
pub use settings::{
    Category, DEFAULT_REMOTE_PORT, DUPLICATE_ANSWERS, MAIN_QUEUE, Queue, RemoteSettings, Schedule,
    Settings, TorrentSettings, default_categories, default_download_dir, default_queues,
    extension_of,
};
pub use store::Store;
pub use task::{
    AddListOptions, AddListResult, AddOptions, ConfirmOptions, Fragments, ListChoices, ListPick,
    Live, MediaChoices, MediaInfo, OnDuplicate, QueueMove, SiteChoices, SiteInfo, SiteOptions,
    Swarm, TaskId, TaskRecord, TaskSnapshot, TaskState, TorrentChoices, TorrentFileChoice,
    TorrentInfo,
};

pub use downpipe_crawl as crawl;
pub use downpipe_http as http;
pub use downpipe_providers as providers;
pub use torrent::{AVAILABLE as TORRENT_SUPPORT, is_torrent_link};
