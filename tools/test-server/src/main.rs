use std::net::SocketAddr;

use clap::Parser;
use downpipe_test_server::{SAMPLES, TestServer};

/// Fault-injection HTTP server for testing and benchmarking Downpipe.
#[derive(Debug, Parser)]
struct Args {
    /// Port on 127.0.0.1 to listen on.
    #[arg(long, default_value_t = 17655)]
    port: u16,
    /// Also serve this folder under /files/, for example HLS or DASH output
    /// of FFmpeg and a page that plays it, to try media detection by hand.
    #[arg(long, value_name = "DIR")]
    files: Option<std::path::PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server = TestServer::bind(SocketAddr::from(([127, 0, 0, 1], args.port))).await?;
    println!("Downpipe test server on http://{}", server.addr);
    if let Some(dir) = args.files {
        println!(
            "Files from {} under {}",
            dir.display(),
            server.url("/files/")
        );
        server.serve_files(dir);
    }
    println!("Links for the browser extension: {}", server.url("/"));
    println!();
    for (path, what) in SAMPLES {
        println!("  {}", server.url(path));
        println!("      {what}");
    }
    println!();
    println!("Streams with checkable fragments (not playable media):");
    println!("  {}", server.url("/hls/master.m3u8?split=1"));
    println!(
        "  {}  (playlists beside the master)",
        server.url("/hls/master.m3u8?split=1&flat=1")
    );
    println!("  {}", server.url("/dash/manifest.mpd?split=1"));
    println!();
    println!("Ctrl+C to stop.");
    server.wait().await;
    Ok(())
}
