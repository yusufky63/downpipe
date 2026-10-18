//! The remote control without the desktop app, for working on the page:
//!
//! ```text
//! pnpm --filter @downpipe/desktop build
//! cargo run -p downpipe-remote --example serve -- <data folder> [port] [dist folder]
//! ```
//!
//! Opens (or creates) a catalog in the data folder, saves downloads in its
//! `downloads` subfolder, listens on 127.0.0.1 and prints the link with a
//! fresh token. Never point it at the data folder of a Downpipe that is
//! running.

use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::sync::Arc;

use downpipe_core::Manager;
use downpipe_remote::{Config, DirAssets, RemoteServer, token};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let Some(data) = args.next().map(PathBuf::from) else {
        eprintln!("usage: serve <data folder> [port] [dist folder]");
        std::process::exit(2);
    };
    let port = args.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let dist = args.next().map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/dist"),
        PathBuf::from,
    );
    std::fs::create_dir_all(&data).expect("data folder");
    let manager = Manager::open(&data).await.expect("open the catalog");
    let settings = manager.settings();
    manager
        .update_settings(downpipe_core::Settings {
            download_dir: data.join("downloads"),
            ..settings
        })
        .expect("download folder");
    let token = token::generate();
    let config = Config {
        bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
        port,
        token_hash: token::parse_hash(&token::hash(&token)).expect("hash"),
    };
    let server = RemoteServer::start(manager.clone(), config, Arc::new(DirAssets::new(dist)))
        .await
        .expect("listen");
    println!("http://{}/#token={token}", server.local_addr());
    let _ = tokio::signal::ctrl_c().await;
    server.stop().await;
    manager.shutdown().await;
}
