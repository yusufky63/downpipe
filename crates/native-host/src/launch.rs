//! Starting the desktop app when the extension sends a download while it is
//! closed, as IDM does.
//!
//! Only an executable in the host's own folder is ever started (installers
//! put the host next to the app, and a development build has both in
//! Cargo's target folder), under one of the fixed names below. Nothing a
//! message carries takes part in choosing it.

use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Passed to the app so it starts in the tray, like the login item does.
pub const HIDDEN_ARG: &str = "--hidden";

/// How long the host waits for a started app to open its pipe. A cold start
/// opens the catalog and re-reads the progress of unfinished downloads.
pub const LAUNCH_WAIT: Duration = Duration::from_secs(10);

/// How often the host looks for the pipe while it waits.
pub const POLL_INTERVAL: Duration = Duration::from_millis(150);

/// The app's executable as the installers name it (Tauri names it after the
/// product) and as Cargo builds it (the package name), in that order.
pub const APP_NAMES: &[&str] = if cfg!(windows) {
    &["Downpipe.exe", "downpipe.exe", "downpipe-desktop.exe"]
} else {
    &["Downpipe", "downpipe", "downpipe-desktop"]
};

/// The app next to `host_exe`, if one of [`APP_NAMES`] is a file there.
pub fn app_beside(host_exe: &Path) -> Option<PathBuf> {
    let dir = host_exe.parent()?;
    APP_NAMES
        .iter()
        .map(|name| dir.join(name))
        .find(|path| path.is_file())
}

/// Start `app` in the tray, detached from the host: no inherited standard
/// streams (stdout carries the Native Messaging protocol, and the app must
/// not write into it), its own process group, and out of the browser's job
/// object where the browser allows that, so closing the port or the
/// browser does not take the app down with the host.
pub fn start(app: &Path) -> io::Result<()> {
    let mut command = Command::new(app);
    command
        .arg(HIDDEN_ARG)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = app.parent() {
        command.current_dir(dir);
    }
    spawn_detached(command)
}

#[cfg(windows)]
fn spawn_detached(mut command: Command) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    let flags = DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP;
    // Firefox puts the host in a job object that it kills with the port.
    // Leaving it needs the job's permission; without it the app still
    // starts, it just ends with the browser.
    command.creation_flags(flags | CREATE_BREAKAWAY_FROM_JOB);
    match command.spawn() {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => {
            command.creation_flags(flags);
            command.spawn().map(drop)
        }
        Err(err) => Err(err),
    }
}

#[cfg(unix)]
fn spawn_detached(mut command: Command) -> io::Result<()> {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
    command.spawn().map(drop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_known_names_next_to_the_host_are_started() {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join(crate::manifest::HOST_EXE);
        std::fs::write(&host, b"").unwrap();
        assert_eq!(app_beside(&host), None, "the host alone");

        // Look-alikes and folders are not the app.
        std::fs::write(dir.path().join("Downpipe-setup.exe"), b"").unwrap();
        std::fs::create_dir(dir.path().join(APP_NAMES[0])).unwrap();
        assert_eq!(app_beside(&host), None);

        // The installers' name wins over Cargo's.
        let cargo = dir.path().join(APP_NAMES[2]);
        std::fs::write(&cargo, b"").unwrap();
        assert_eq!(app_beside(&host), Some(cargo));
        std::fs::remove_dir(dir.path().join(APP_NAMES[0])).unwrap();
        let installed = dir.path().join(APP_NAMES[0]);
        std::fs::write(&installed, b"").unwrap();
        assert_eq!(app_beside(&host), Some(installed));

        // An app elsewhere, even one folder up, is never found.
        let nested = dir.path().join("sub").join(crate::manifest::HOST_EXE);
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        assert_eq!(app_beside(&nested), None);
    }

    #[test]
    fn a_missing_app_fails_to_start() {
        let dir = tempfile::tempdir().unwrap();
        assert!(start(&dir.path().join(APP_NAMES[0])).is_err());
    }
}
