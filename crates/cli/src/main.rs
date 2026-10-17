use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use downpipe_http::{
    Checksum, DownloadRequest, Engine, EngineConfig, Outcome, Phase, RateLimiter, meta, parse_rate,
};
use indicatif::{ProgressBar, ProgressStyle};

mod format;

use format::{bytes, duration};

/// Exit status when a download was paused with Ctrl+C.
const EXIT_PAUSED: u8 = 2;

/// Downpipe: an adaptive, resumable multi-connection downloader.
#[derive(Debug, Parser)]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download a URL, resuming automatically if an unfinished copy exists.
    Get(GetArgs),
    /// Continue a download from its `.part` or `.part.meta` file.
    Resume {
        /// Path to `<name>.part`, `<name>.part.meta` or the final `<name>`.
        path: PathBuf,
        #[command(flatten)]
        tuning: Tuning,
    },
    /// Show what the server reports about a URL without downloading it.
    Probe {
        url: String,
        /// Extra request header, e.g. `-H "Referer: https://example.org/"`.
        #[arg(short = 'H', long = "header", value_name = "NAME: VALUE")]
        headers: Vec<String>,
    },
}

#[derive(Debug, Args)]
struct GetArgs {
    url: String,
    /// Folder to save into.
    #[arg(short, long, value_name = "DIR", default_value = ".")]
    output: PathBuf,
    /// File name to use instead of the one from the server.
    #[arg(short = 'n', long = "name")]
    file_name: Option<String>,
    /// Expected digest, e.g. `sha256:9f86d0...`. Checked before the file is renamed into place.
    #[arg(long)]
    checksum: Option<String>,
    /// Extra request header, e.g. `-H "Referer: https://example.org/"`.
    #[arg(short = 'H', long = "header", value_name = "NAME: VALUE")]
    headers: Vec<String>,
    #[command(flatten)]
    tuning: Tuning,
}

#[derive(Debug, Args)]
struct Tuning {
    /// Maximum parallel connections (1-32).
    #[arg(short = 'c', long, default_value_t = 8, value_parser = clap::value_parser!(u16).range(1..=32))]
    connections: u16,
    /// Connections to start with before measuring whether more help.
    #[arg(long, default_value_t = 2)]
    initial_connections: u16,
    /// Speed limit, e.g. `500K`, `2M`, `1.5MB`.
    #[arg(long, value_name = "RATE")]
    limit: Option<String>,
    /// Proxy URL (`http://`, `https://` or `socks5://`).
    #[arg(long)]
    proxy: Option<String>,
    /// No progress bar.
    #[arg(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DOWNPIPE_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .with_writer(std::io::stderr)
        .with_target(false)
        .init();

    match run(Cli::parse()).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Get(args) => {
            let mut req = DownloadRequest::new(args.url, args.output);
            req.file_name = args.file_name;
            req.checksum = args
                .checksum
                .as_deref()
                .map(str::parse::<Checksum>)
                .transpose()?;
            req.headers = parse_headers(&args.headers)?;
            download(req, &args.tuning).await
        }
        Command::Resume { path, tuning } => {
            let final_path = meta::final_path_for(&path).unwrap_or(path);
            let meta_path = meta::meta_path(&final_path);
            let saved = meta::load(&meta_path)
                .with_context(|| format!("no resume data at {}", meta_path.display()))?;
            let dir = final_path
                .parent()
                .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
            let mut req = DownloadRequest::new(saved.url.clone(), dir);
            req.file_name = Some(saved.file_name.clone());
            req.checksum = saved.checksum.as_deref().and_then(|c| c.parse().ok());
            download(req, &tuning).await
        }
        Command::Probe { url, headers } => {
            let engine = Engine::new(EngineConfig::default())?;
            let info = engine.probe(&url, &parse_headers(&headers)?).await?;
            println!("URL           {}", info.final_url);
            println!("File name     {}", info.file_name);
            println!(
                "Size          {}",
                info.total_size.map_or_else(|| "unknown".to_string(), bytes)
            );
            println!(
                "Byte ranges   {}",
                if info.accepts_ranges { "yes" } else { "no" }
            );
            println!(
                "Resumable     {}",
                if info.is_resumable() {
                    "yes"
                } else {
                    "no (one connection, restarts from zero)"
                }
            );
            if let Some(t) = &info.content_type {
                println!("Content type  {t}");
            }
            if let Some(e) = &info.etag {
                println!("ETag          {e}");
            }
            if let Some(m) = &info.last_modified {
                println!("Last-Modified {m}");
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

async fn download(mut req: DownloadRequest, tuning: &Tuning) -> Result<ExitCode> {
    let limit = match tuning.limit.as_deref() {
        Some(raw) => {
            parse_rate(raw).with_context(|| format!("`{raw}` is not a speed like 2M or 500K"))?
        }
        None => 0,
    };
    let config = EngineConfig {
        max_connections: usize::from(tuning.connections),
        initial_connections: usize::from(tuning.initial_connections.clamp(1, tuning.connections)),
        proxy: tuning.proxy.clone(),
        ..EngineConfig::default()
    };
    req.max_connections = Some(config.max_connections);
    let engine = Engine::with_limiter(config, Arc::new(RateLimiter::new(limit)))?;

    let started = Instant::now();
    let handle = engine.start(req);
    let control = handle.control();
    let mut rx = handle.progress();

    // First Ctrl+C pauses cleanly (resume data is written); a second one quits.
    tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            control.pause();
            if tokio::signal::ctrl_c().await.is_ok() {
                std::process::exit(130);
            }
        }
    });

    let bar = if tuning.quiet {
        ProgressBar::hidden()
    } else {
        let bar = ProgressBar::new(0);
        bar.set_style(
            ProgressStyle::with_template("{bar:32.cyan/blue} {percent:>3}%  {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_bar())
                .progress_chars("##-"),
        );
        bar.enable_steady_tick(Duration::from_millis(200));
        bar
    };

    let watcher = {
        let bar = bar.clone();
        tokio::spawn(async move {
            let mut shown_notices = 0;
            while rx.changed().await.is_ok() {
                let p = rx.borrow_and_update().clone();
                for notice in p.notices.iter().skip(shown_notices) {
                    bar.println(format!("note: {notice}"));
                }
                shown_notices = p.notices.len();
                if let Some(total) = p.total {
                    bar.set_length(total);
                }
                bar.set_position(p.downloaded);
                let name = p.file_name.as_deref().unwrap_or("connecting");
                let status = match p.phase {
                    Phase::Starting => "starting".to_string(),
                    Phase::Waiting => "waiting to retry".to_string(),
                    Phase::Verifying => "verifying checksum".to_string(),
                    Phase::Finalizing | Phase::Done => "finishing".to_string(),
                    Phase::Downloading => {
                        let eta = p
                            .eta()
                            .map(|d| format!("  ETA {}", duration(d)))
                            .unwrap_or_default();
                        format!("{}/s  {} conn{eta}", bytes(p.speed), p.connections)
                    }
                };
                let size = p.total.map_or_else(
                    || bytes(p.downloaded),
                    |t| format!("{} / {}", bytes(p.downloaded), bytes(t)),
                );
                bar.set_message(format!("{name}  {size}  {status}"));
            }
        })
    };

    let result = handle.wait().await;
    let _ = watcher.await;
    bar.finish_and_clear();

    match result? {
        Outcome::Completed {
            path,
            bytes: size,
            verified,
        } => {
            let elapsed = started.elapsed();
            let avg = (size as f64 / elapsed.as_secs_f64().max(0.001)) as u64;
            println!(
                "Saved {} ({}) in {}, {}/s average",
                path.display(),
                bytes(size),
                duration(elapsed),
                bytes(avg)
            );
            if let Some(c) = verified {
                println!("{} verified", c.algo.name());
            }
            Ok(ExitCode::SUCCESS)
        }
        Outcome::Paused {
            downloaded,
            total,
            resumable,
            ..
        } => {
            if resumable {
                let pct = total
                    .filter(|t| *t > 0)
                    .map(|t| format!(" ({:.0}%)", downloaded as f64 * 100.0 / t as f64))
                    .unwrap_or_default();
                println!(
                    "Paused at {}{pct}. Run the same command again to continue.",
                    bytes(downloaded)
                );
            } else {
                println!(
                    "Stopped. This server does not support resuming, so the next run starts from zero."
                );
            }
            Ok(ExitCode::from(EXIT_PAUSED))
        }
        Outcome::Canceled => {
            println!("Canceled.");
            Ok(ExitCode::from(EXIT_PAUSED))
        }
    }
}

fn parse_headers(raw: &[String]) -> Result<Vec<(String, String)>> {
    raw.iter()
        .map(|h| match h.split_once(':') {
            Some((name, value)) if !name.trim().is_empty() => {
                Ok((name.trim().to_string(), value.trim().to_string()))
            }
            _ => bail!("header `{h}` must look like `Name: value`"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers() {
        let parsed = parse_headers(&["Referer: https://a.example/x:y".into()]).unwrap();
        assert_eq!(
            parsed,
            vec![("Referer".into(), "https://a.example/x:y".into())]
        );
        assert!(parse_headers(&["nonsense".into()]).is_err());
        assert!(parse_headers(&[": empty".into()]).is_err());
    }

    #[test]
    fn cli_parses() {
        let cli = Cli::try_parse_from([
            "downpipe",
            "get",
            "https://example.org/a",
            "-c",
            "4",
            "--limit",
            "2M",
        ])
        .unwrap();
        let Command::Get(args) = cli.command else {
            panic!("expected get");
        };
        assert_eq!(args.tuning.connections, 4);
        assert_eq!(args.tuning.limit.as_deref(), Some("2M"));
        assert!(Cli::try_parse_from(["downpipe", "get", "u", "-c", "99"]).is_err());
    }
}
