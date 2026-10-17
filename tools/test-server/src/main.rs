use std::net::SocketAddr;

use clap::Parser;
use downpipe_test_server::{SAMPLES, TestServer};

/// Fault-injection HTTP server for testing and benchmarking Downpipe.
#[derive(Debug, Parser)]
struct Args {
    /// Port on 127.0.0.1 to listen on.
    #[arg(long, default_value_t = 17655)]
    port: u16,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let server = TestServer::bind(SocketAddr::from(([127, 0, 0, 1], args.port))).await?;
    println!("Downpipe test server on http://{}", server.addr);
    println!("Links for the browser extension: {}", server.url("/"));
    println!();
    for (path, what) in SAMPLES {
        println!("  {}", server.url(path));
        println!("      {what}");
    }
    println!();
    println!("Ctrl+C to stop.");
    server.wait().await;
    Ok(())
}
