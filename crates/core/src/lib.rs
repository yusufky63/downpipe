//! Downpipe's application core: a persistent download queue on top of
//! [`downpipe_http`]. Desktop, CLI and future web UIs all drive this.

pub mod backup;
pub mod feeds;
pub mod idm;
mod manager;
pub mod rules;
pub mod schedule;
mod settings;
mod store;
mod task;

pub use manager::{CoreError, Manager, ManagerEvent};
pub use rules::{RefusedHost, RuleProblem, RuleProxy, RuleTest, SiteRule};
pub use settings::{
    Category, DEFAULT_REMOTE_PORT, DUPLICATE_ANSWERS, MAIN_QUEUE, Queue, RemoteSettings, Schedule,
    Settings, default_categories, default_download_dir, default_queues, extension_of,
};
pub use store::Store;
pub use task::{
    AddOptions, ConfirmOptions, Fragments, Live, MediaChoices, MediaInfo, OnDuplicate, QueueMove,
    SiteChoices, SiteInfo, SiteOptions, TaskId, TaskRecord, TaskSnapshot, TaskState,
};

pub use downpipe_crawl as crawl;
pub use downpipe_http as http;
pub use downpipe_providers as providers;
