//! Sleep and shut down the computer for "when all downloads finish", and
//! keep it awake while downloads run. Fixed commands with argument arrays:
//! nothing user-supplied reaches them.

use std::io;
use std::process::Command;

pub fn sleep() -> io::Result<()> {
    #[cfg(windows)]
    let status = Command::new(system32("rundll32.exe"))
        .args(["powrprof.dll,SetSuspendState", "0,1,0"])
        .status()?;
    #[cfg(target_os = "macos")]
    let status = Command::new("/usr/bin/pmset").arg("sleepnow").status()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("systemctl").arg("suspend").status()?;
    check(status)
}

pub fn shutdown() -> io::Result<()> {
    #[cfg(windows)]
    let status = Command::new(system32("shutdown.exe"))
        .args(["/s", "/t", "0"])
        .status()?;
    #[cfg(target_os = "macos")]
    let status = Command::new("/usr/bin/osascript")
        .args(["-e", "tell application \"System Events\" to shut down"])
        .status()?;
    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("systemctl").arg("poweroff").status()?;
    check(status)
}

fn check(status: std::process::ExitStatus) -> io::Result<()> {
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("the system refused ({status})")))
    }
}

/// Absolute path, so a program of the same name in the working directory
/// or on PATH is never started instead.
#[cfg(windows)]
fn system32(program: &str) -> std::path::PathBuf {
    let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    std::path::PathBuf::from(root)
        .join("System32")
        .join(program)
}

/// Keeps the computer from sleeping while downloads run. The display may
/// still turn off.
///
/// On Windows the request belongs to the thread that made it, so one thread
/// owns it for the life of the app instead of whichever async worker
/// happens to run the background loop.
pub struct KeepAwake {
    tx: std::sync::mpsc::Sender<bool>,
    on: std::sync::atomic::AtomicBool,
}

impl KeepAwake {
    pub fn spawn(app_id: String) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<bool>();
        let started = std::thread::Builder::new()
            .name("keep-awake".into())
            .spawn(move || {
                let mut guard = None;
                for on in rx {
                    if !on {
                        guard = None;
                        continue;
                    }
                    if guard.is_some() {
                        continue;
                    }
                    match keepawake::Builder::default()
                        .idle(true)
                        .reason("Downloading files")
                        .app_name("Downpipe")
                        .app_reverse_domain(app_id.clone())
                        .create()
                    {
                        Ok(awake) => guard = Some(awake),
                        Err(err) => tracing::warn!("could not keep the computer awake: {err}"),
                    }
                }
            });
        if let Err(err) = started {
            tracing::warn!("keeping the computer awake is off: {err}");
        }
        Self {
            tx,
            on: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn set(&self, on: bool) {
        use std::sync::atomic::Ordering;
        if self.on.swap(on, Ordering::Relaxed) != on {
            let _ = self.tx.send(on);
        }
    }
}
