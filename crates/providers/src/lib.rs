//! Site providers: the video on a web page, where the page's player needs
//! site-specific work that a plain HTTP download cannot do.
//!
//! YouTube no longer serves its videos as files a download manager can
//! intercept (its web player streams over SABR, and links need PO tokens and
//! answers to JavaScript challenges), so Downpipe hands such pages to
//! [yt-dlp](https://github.com/yt-dlp/yt-dlp), an external program the user
//! installs or lets Downpipe download, and runs it as a separate process:
//!
//! * [`pages`] decides which links are such pages.
//! * [`info`] runs `yt-dlp -J` and turns its answer into the quality list
//!   the stream chooser already shows.
//! * [`job`] runs the download behind a normal
//!   [`DownloadHandle`](downpipe_http::DownloadHandle), with yt-dlp's
//!   progress turned into the usual [`Progress`](downpipe_http::Progress).
//! * [`tools`] finds yt-dlp, Deno (or Node) and FFmpeg; [`install`]
//!   downloads yt-dlp and Deno when the user asks.
//!
//! Nothing here sends cookies, reads yt-dlp's configuration files or runs a
//! shell. Protected (DRM) and live videos are refused.

mod args;
mod errors;
pub mod files;
pub mod info;
pub mod install;
pub mod job;
pub mod pages;
mod process;
pub mod progress;
pub mod tools;

pub use args::{format_selector, needs_merge, valid_format_id};
pub use info::PageInfo;
pub use job::SiteRequest;
pub use pages::Page;
pub use tools::{ToolConfig, Tools, ToolsStatus};
