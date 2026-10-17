use std::net::SocketAddr;

use clap::Parser;
use downpipe_test_server::TestServer;

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
    println!();
    println!("  {}", server.url("/file/104857600"));
    println!("      100 MiB, ranges + ETag");
    println!("  {}", server.url("/file/52428800?ranges=0"));
    println!("      no range support (single connection)");
    println!("  {}", server.url("/file/209715200?rate=4194304"));
    println!("      4 MiB/s per connection, so more connections help");
    println!(
        "  {}",
        server.url("/file/52428800?drop_after=3000000&drop_times=4")
    );
    println!("      first four responses die mid-stream");
    println!(
        "  {}",
        server.url("/file/20971520?fail=503&fail_times=2&retry_after=1")
    );
    println!("      503 with Retry-After twice, then OK");
    println!(
        "  {}",
        server.url("/file/10485760?name=r%C3%A9sum%C3%A9.pdf&type=application/pdf")
    );
    println!("      Content-Disposition with a UTF-8 name");
    println!();
    println!("Ctrl+C to stop.");
    server.wait().await;
    Ok(())
}
