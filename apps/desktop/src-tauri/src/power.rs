//! Sleep and shut down the computer for "when all downloads finish".
//! Fixed commands with argument arrays: nothing user-supplied reaches them.

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
