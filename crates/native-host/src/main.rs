//! `downpipe-native-host`: started by the browser for the Downpipe extension,
//! or run by hand to register itself with browsers.

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use downpipe_native_host::host::{self, PipeLink};
use downpipe_native_host::ipc::Endpoint;
use downpipe_native_host::manifest::{
    self, Browser, CHROMIUM_EXTENSION_ID, FIREFOX_EXTENSION_ID, Layout, Owner,
};

/// Browser bridge for Downpipe. Browsers start it with an extension origin as
/// the first argument; the commands below register it with them.
#[derive(Debug, Parser)]
#[command(name = "downpipe-native-host", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Register this host for the current user (no administrator rights needed).
    Install(Target),
    /// Remove the registration.
    Uninstall {
        /// Browsers to unregister. Default: all.
        #[arg(long = "browser", value_enum)]
        browsers: Vec<Browser>,
        /// Only remove a registration that starts the host at this path, and
        /// keep one that starts another copy. The desktop uninstallers pass
        /// the host they are about to delete.
        #[arg(long, value_name = "PATH")]
        only_if_host: Option<PathBuf>,
    },
    /// Show which browsers have the host registered.
    Status,
    /// Print the manifest for one browser instead of installing it.
    Manifest {
        #[arg(long, value_enum)]
        browser: Browser,
        #[command(flatten)]
        ids: Ids,
        /// Path written into the manifest. Default: this executable.
        #[arg(long)]
        host_path: Option<PathBuf>,
    },
}

#[derive(Debug, Args)]
struct Target {
    /// Browsers to register with. Default: Chrome, Edge, Chromium and Firefox
    /// (on macOS and Linux only those with a profile folder).
    #[arg(long = "browser", value_enum)]
    browsers: Vec<Browser>,
    #[command(flatten)]
    ids: Ids,
    /// Path written into the manifest. Default: this executable.
    #[arg(long)]
    host_path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct Ids {
    /// Chrome/Edge extension id allowed to start the host (repeatable).
    /// Default: the id of the Downpipe extension.
    #[arg(long = "extension-id")]
    extension_ids: Vec<String>,
    /// Firefox add-on id allowed to start the host (repeatable).
    #[arg(long = "firefox-extension-id")]
    firefox_ids: Vec<String>,
}

impl Ids {
    fn for_browser(&self, browser: Browser) -> Vec<String> {
        let (given, default) = if browser.is_firefox() {
            (&self.firefox_ids, FIREFOX_EXTENSION_ID)
        } else {
            (&self.extension_ids, CHROMIUM_EXTENSION_ID)
        };
        if given.is_empty() {
            vec![default.to_string()]
        } else {
            given.clone()
        }
    }
}

const COMMANDS: &[&str] = &["install", "uninstall", "status", "manifest", "help"];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let wants_cli = args
        .first()
        .is_some_and(|a| COMMANDS.contains(&a.as_str()) || a.starts_with('-'));
    if wants_cli {
        return cli(Cli::parse());
    }
    if args.is_empty() && std::io::stdin().is_terminal() {
        eprintln!(
            "downpipe-native-host is started by your browser for the Downpipe extension.\n\
             To register it, run: downpipe-native-host install\n\
             More: downpipe-native-host --help"
        );
        return ExitCode::from(2);
    }
    serve_browser(&args)
}

/// Native Messaging mode. stdout carries the protocol, so nothing else may
/// be printed there; diagnostics go to stderr, which browsers log.
fn serve_browser(args: &[String]) -> ExitCode {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("DOWNPIPE_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();
    let origin = host::origin_from_args(args);
    let link = PipeLink {
        endpoint: Endpoint::from_env_or_default(),
        timeout: host::APP_TIMEOUT,
    };
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    match host::run(
        &mut stdin.lock(),
        &mut stdout.lock(),
        &link,
        origin.as_deref(),
    ) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("native messaging session ended: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cli(cli: Cli) -> ExitCode {
    let result = match cli.command {
        Command::Install(target) => install(target),
        Command::Uninstall {
            browsers,
            only_if_host,
        } => uninstall(browsers, only_if_host),
        Command::Status => status(),
        Command::Manifest {
            browser,
            ids,
            host_path,
        } => host_path_or_self(host_path).and_then(|path| {
            let value = manifest::manifest(browser, &path, &ids.for_browser(browser))?;
            println!("{}", serde_json::to_string_pretty(&value)?);
            Ok(())
        }),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

type CliResult = Result<(), Box<dyn std::error::Error>>;

fn host_path_or_self(path: Option<PathBuf>) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match path {
        Some(path) if path.is_absolute() => Ok(path),
        Some(path) => Ok(std::env::current_dir()?.join(path)),
        // Not canonicalized: on Windows that adds a `\\?\` prefix browsers reject.
        None => Ok(std::env::current_exe()?),
    }
}

fn install(target: Target) -> CliResult {
    let layout = Layout::current()?;
    let host_path = host_path_or_self(target.host_path)?;
    let explicit = !target.browsers.is_empty();
    let browsers = if explicit {
        target.browsers
    } else {
        Browser::ALL.to_vec()
    };
    let mut installed = 0;
    for browser in browsers {
        if !explicit && !layout.browser_present(browser) {
            println!("{:<9} skipped (no profile folder found)", browser.name());
            continue;
        }
        let value = manifest::manifest(browser, &host_path, &target.ids.for_browser(browser))?;
        let registration = manifest::install(&layout, browser, &value)?;
        installed += 1;
        match registration.registry_key {
            Some(key) => println!(
                "{:<9} HKCU\\{key}\n          -> {}",
                browser.name(),
                registration.manifest_path.display()
            ),
            None => println!(
                "{:<9} {}",
                browser.name(),
                registration.manifest_path.display()
            ),
        }
    }
    if installed > 0 {
        println!("\nHost: {}", host_path.display());
        println!("Restart the browser if the extension still reports that the bridge is missing.");
    }
    Ok(())
}

fn uninstall(browsers: Vec<Browser>, only_if_host: Option<PathBuf>) -> CliResult {
    let layout = Layout::current()?;
    let browsers = if browsers.is_empty() {
        Browser::ALL.to_vec()
    } else {
        browsers
    };
    let host = only_if_host
        .map(|path| host_path_or_self(Some(path)))
        .transpose()?;
    let mut failed = 0;
    // One browser failing does not keep the others registered.
    for browser in browsers {
        let outcome = match &host {
            None => manifest::uninstall(&layout, browser)
                .map(|removed| if removed { "removed" } else { "not registered" }.to_string()),
            Some(host) => manifest::uninstall_host(&layout, browser, host).map(describe_owner),
        };
        match outcome {
            Ok(outcome) => println!("{:<9} {outcome}", browser.name()),
            Err(e) => {
                failed += 1;
                println!("{:<9} failed: {e}", browser.name());
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} browser registration(s) could not be removed").into());
    }
    Ok(())
}

fn describe_owner(owner: Owner) -> String {
    match owner {
        Owner::This => "removed".to_string(),
        Owner::Nobody => "not registered".to_string(),
        Owner::Other(path) => format!("kept, it starts {}", path.display()),
        Owner::Unknown => "kept, the registered manifest cannot be read".to_string(),
    }
}

fn status() -> CliResult {
    let layout = Layout::current()?;
    for browser in Browser::ALL {
        match manifest::registered_path(&layout, browser)? {
            Some(path) => println!("{:<9} {}", browser.name(), path.display()),
            None => println!("{:<9} not registered", browser.name()),
        }
    }
    println!("\nApp endpoint: {}", Endpoint::from_env_or_default());
    Ok(())
}
