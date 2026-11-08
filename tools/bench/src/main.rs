//! Repeatable download benchmarks.
//!
//! Each run gets a fresh fault-injection server (so counters such as
//! `drop_times` start over) and an empty folder, downloads one scenario, and
//! checks every byte of the result. Downpipe's engine runs in this process;
//! other programs (aria2c, an older Downpipe CLI) can be compared with
//! `--external`. Contenders take turns going first from round to round, so a
//! machine that gets busier over time does not favour one of them.
//!
//! The servers run on loopback, so the numbers show how a program copes with
//! server behaviour (per-connection caps, slow paths, refusals), not how fast
//! a real network is.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use clap::Parser;
use downpipe_http::{DownloadRequest, Engine, EngineConfig};
use downpipe_test_server::{TestServer, byte_at};

/// One situation a download manager meets, as the test server plays it.
struct Scenario {
    name: &'static str,
    what: &'static str,
    size: u64,
    query: &'static str,
}

const MIB: u64 = 1024 * 1024;

const SCENARIOS: &[Scenario] = &[
    Scenario {
        name: "fast-1g",
        what: "1 GiB over loopback, no limits",
        size: 1024 * MIB,
        query: "",
    },
    Scenario {
        name: "capped",
        what: "100 MiB, 2 MiB/s per connection",
        size: 100 * MIB,
        query: "rate=2097152",
    },
    Scenario {
        name: "slow-tail",
        what: "100 MiB, 4 MiB/s per connection, the last 15 MiB at 256 KiB/s",
        size: 100 * MIB,
        query: "rate=4194304&slow_from=89128960&slow_rate=262144",
    },
    Scenario {
        name: "crawl",
        what: "32 MiB, 4 MiB/s per connection, every third connection at 128 KiB/s",
        size: 32 * MIB,
        query: "rate=4194304&crawl_every=3&crawl_rate=131072",
    },
    Scenario {
        name: "host-limit",
        what: "100 MiB, 2 MiB/s per connection, at most 4 connections (429 beyond)",
        size: 100 * MIB,
        query: "rate=2097152&max_conns=4&limit_status=429",
    },
    Scenario {
        name: "drops",
        what: "50 MiB, 4 MiB/s per connection, the first 6 responses cut after 3 MB",
        size: 50 * MIB,
        query: "rate=4194304&drop_after=3000000&drop_times=6",
    },
    Scenario {
        name: "stall",
        what: "50 MiB, 4 MiB/s per connection, one response goes silent at 20 MB",
        size: 50 * MIB,
        query: "rate=4194304&stall_at=20000000&stall_times=1",
    },
];

/// Benchmark Downpipe's engine, and optionally other programs, on the
/// fault-injection server.
#[derive(Debug, Parser)]
struct Args {
    /// Runs per scenario and contender.
    #[arg(long, default_value_t = 5)]
    rounds: usize,
    /// Only these scenarios (repeatable); `--list` shows them.
    #[arg(long = "scenario", value_name = "NAME")]
    scenarios: Vec<String>,
    /// Another program to compare, as `NAME=COMMAND`. `{url}` and `{dir}` in
    /// the command are replaced; it must save one file into `{dir}`. Example:
    /// `aria2=aria2c -x16 -s16 -d {dir} {url}`.
    #[arg(long = "external", value_name = "NAME=COMMAND")]
    externals: Vec<String>,
    /// Leave Downpipe's engine out (compare externals only).
    #[arg(long)]
    no_engine: bool,
    /// Give up on a run after this many seconds.
    #[arg(long, default_value_t = 240)]
    timeout: u64,
    /// List the scenarios and exit.
    #[arg(long)]
    list: bool,
}

enum Contender {
    Engine,
    External { name: String, command: String },
}

impl Contender {
    fn name(&self) -> &str {
        match self {
            Self::Engine => "downpipe",
            Self::External { name, .. } => name,
        }
    }

    async fn run(&self, url: &str, dir: &Path) -> Result<()> {
        match self {
            Self::Engine => {
                let engine = Engine::new(EngineConfig::default())?;
                engine.start(DownloadRequest::new(url, dir)).wait().await?;
                Ok(())
            }
            Self::External { command, .. } => {
                let dir_text = dir.to_string_lossy();
                let mut parts = command
                    .split_whitespace()
                    .map(|p| p.replace("{url}", url).replace("{dir}", &dir_text));
                let program = parts.next().context("empty command")?;
                let status = tokio::process::Command::new(program)
                    .args(parts)
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .status()
                    .await?;
                if !status.success() {
                    bail!("exited with {status}");
                }
                Ok(())
            }
        }
    }
}

#[derive(Default)]
struct Samples {
    times: Vec<Duration>,
    failures: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    if args.list {
        for s in SCENARIOS {
            println!("{:<11} {}", s.name, s.what);
        }
        return Ok(());
    }
    let scenarios: Vec<&Scenario> = SCENARIOS
        .iter()
        .filter(|s| args.scenarios.is_empty() || args.scenarios.iter().any(|n| n == s.name))
        .collect();
    if scenarios.is_empty() {
        bail!("no such scenario; --list shows them");
    }
    let mut contenders = Vec::new();
    if !args.no_engine {
        contenders.push(Contender::Engine);
    }
    for spec in &args.externals {
        let (name, command) = spec
            .split_once('=')
            .with_context(|| format!("`{spec}` is not NAME=COMMAND"))?;
        contenders.push(Contender::External {
            name: name.trim().to_string(),
            command: command.trim().to_string(),
        });
    }
    if contenders.is_empty() {
        bail!("nothing to measure");
    }

    let timeout = Duration::from_secs(args.timeout);
    let mut results: Vec<Vec<Samples>> = scenarios
        .iter()
        .map(|_| contenders.iter().map(|_| Samples::default()).collect())
        .collect();
    for round in 0..args.rounds {
        for (si, scenario) in scenarios.iter().enumerate() {
            for k in 0..contenders.len() {
                let ci = (k + round) % contenders.len();
                let contender = &contenders[ci];
                let outcome = measure(scenario, contender, timeout).await;
                let samples = &mut results[si][ci];
                match outcome {
                    Ok(took) => {
                        eprintln!(
                            "round {} {:<11} {:<10} {:>8.2} s",
                            round + 1,
                            scenario.name,
                            contender.name(),
                            took.as_secs_f64()
                        );
                        samples.times.push(took);
                    }
                    Err(e) => {
                        eprintln!(
                            "round {} {:<11} {:<10} failed: {e:#}",
                            round + 1,
                            scenario.name,
                            contender.name()
                        );
                        samples.failures.push(format!("{e:#}"));
                    }
                }
            }
        }
    }
    print_table(&scenarios, &contenders, &results, args.rounds);
    Ok(())
}

async fn measure(
    scenario: &Scenario,
    contender: &Contender,
    timeout: Duration,
) -> Result<Duration> {
    let mut server = TestServer::start().await?;
    let dir = tempfile::tempdir()?;
    let path = format!("/file/{}?{}", scenario.size, scenario.query);
    let url = server.url(path.trim_end_matches('?'));
    let started = Instant::now();
    let run = tokio::time::timeout(timeout, contender.run(&url, dir.path())).await;
    let took = started.elapsed();
    server.stop().await;
    match run {
        Err(_) => bail!("no result within {timeout:?}"),
        Ok(r) => r?,
    }
    let file = only_file(dir.path())?;
    verify(&file, scenario.size)?;
    Ok(took)
}

/// The one file a run left behind, ignoring resume data.
fn only_file(dir: &Path) -> Result<PathBuf> {
    let files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_file())
        .filter(|p| {
            let name = p.to_string_lossy();
            !(name.ends_with(".part") || name.ends_with(".meta") || name.ends_with(".aria2"))
        })
        .collect();
    match files.as_slice() {
        [one] => Ok(one.clone()),
        [] => bail!("no file was saved"),
        _ => bail!("{} files were saved", files.len()),
    }
}

/// Every byte must be the server's: the content is a function of the offset.
fn verify(path: &Path, size: u64) -> Result<()> {
    use std::io::Read;
    let len = std::fs::metadata(path)?.len();
    if len != size {
        bail!("{len} bytes instead of {size}");
    }
    let mut file = std::io::BufReader::with_capacity(1 << 20, std::fs::File::open(path)?);
    let mut buf = vec![0u8; 1 << 20];
    let mut offset = 0u64;
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        for (i, b) in buf[..n].iter().enumerate() {
            let at = offset + i as u64;
            if *b != byte_at(at, 0) {
                bail!("wrong data at byte {at}");
            }
        }
        offset += n as u64;
    }
}

fn print_table(
    scenarios: &[&Scenario],
    contenders: &[Contender],
    results: &[Vec<Samples>],
    rounds: usize,
) {
    println!();
    println!("{rounds} round(s); median seconds (interquartile range); failed runs in brackets.");
    println!();
    let mut header = String::from("| scenario |");
    let mut rule = String::from("|---|");
    for c in contenders {
        header.push_str(&format!(" {} |", c.name()));
        rule.push_str("---|");
    }
    println!("{header}");
    println!("{rule}");
    for (si, scenario) in scenarios.iter().enumerate() {
        let mut row = format!("| {} |", scenario.name);
        for samples in &results[si] {
            row.push_str(&format!(" {} |", cell(samples)));
        }
        println!("{row}");
    }
    println!();
    for s in scenarios {
        println!("- `{}`: {}", s.name, s.what);
    }
    for (si, scenario) in scenarios.iter().enumerate() {
        for (ci, samples) in results[si].iter().enumerate() {
            if let Some(first) = samples.failures.first() {
                println!("- {} / {}: {first}", scenario.name, contenders[ci].name());
            }
        }
    }
}

fn cell(samples: &Samples) -> String {
    let failed = if samples.failures.is_empty() {
        String::new()
    } else {
        format!(" [{}]", samples.failures.len())
    };
    if samples.times.is_empty() {
        return format!("-{failed}");
    }
    let mut secs: Vec<f64> = samples.times.iter().map(Duration::as_secs_f64).collect();
    secs.sort_by(f64::total_cmp);
    let q = |p: f64| {
        let i = (p * (secs.len() - 1) as f64).round() as usize;
        secs[i]
    };
    format!("{:.2} ({:.2}-{:.2}){failed}", q(0.5), q(0.25), q(0.75))
}
