//! Starting and stopping external programs.
//!
//! Programs run from argument lists, never a shell string, without a console
//! window, and with stdin closed. yt-dlp starts helpers of its own (the
//! Windows build unpacks itself into a second process, and runs FFmpeg and
//! Deno), so stopping it means stopping all of them: on Windows the process
//! is put in a job object that ends every process in it, on Unix in a
//! process group of its own.
//!
//! The core's post-download actions (a command, a virus scanner) start their
//! programs the same way.

use std::path::Path;
use std::process::Stdio;

/// A command for `program` with the settings every external tool gets.
pub fn command(program: &Path) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(program);
    command
        .stdin(Stdio::null())
        // yt-dlp is Python: make its output UTF-8 whatever the console's
        // code page is.
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        // No console window flashing up from a GUI app.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    command.process_group(0);
    command
}

/// The process tree of one started program.
#[derive(Debug)]
pub struct Tree {
    #[cfg(windows)]
    job: Option<windows::Job>,
    #[cfg(unix)]
    pid: Option<u32>,
}

impl Tree {
    /// Take charge of `child` and everything it starts from now on.
    pub fn adopt(child: &tokio::process::Child) -> Self {
        #[cfg(windows)]
        {
            let job = child.raw_handle().and_then(|handle| {
                windows::Job::new()
                    .and_then(|job| job.assign(handle).map(|()| job))
                    .map_err(|e| tracing::debug!("no job object for the process: {e}"))
                    .ok()
            });
            Self { job }
        }
        #[cfg(unix)]
        {
            Self { pid: child.id() }
        }
    }

    /// End the program and every process it started.
    pub fn kill(&self, child: &mut tokio::process::Child) {
        #[cfg(windows)]
        if let Some(job) = &self.job {
            job.terminate();
        }
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            // The child leads its own group (`process_group(0)`), so this
            // reaches FFmpeg and anything else it started. Best effort: the
            // child itself is killed below either way.
            let _ = std::process::Command::new("kill")
                .args(["-KILL", "--", &format!("-{pid}")])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = child.start_kill();
    }
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::io::RawHandle;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject, TerminateJobObject,
    };

    /// A job object that kills its processes when closed, so nothing
    /// outlives Downpipe either.
    #[derive(Debug)]
    pub(super) struct Job(HANDLE);

    // SAFETY: a job handle may be used and closed from any thread.
    unsafe impl Send for Job {}
    // SAFETY: the handle is only passed to thread-safe kernel calls.
    unsafe impl Sync for Job {}

    impl Job {
        pub(super) fn new() -> io::Result<Self> {
            // SAFETY: both pointers may be null (default security, no name).
            let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }
            let job = Self(handle);
            // SAFETY: the struct is plain data; all zeroes is a valid value.
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            // SAFETY: `info` lives across the call and the size matches its type.
            let ok = unsafe {
                SetInformationJobObject(
                    job.0,
                    JobObjectExtendedLimitInformation,
                    (&raw const info).cast(),
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if ok == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(job)
        }

        pub(super) fn assign(&self, process: RawHandle) -> io::Result<()> {
            // SAFETY: `process` is the live handle of a child we started.
            if unsafe { AssignProcessToJobObject(self.0, process) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }

        pub(super) fn terminate(&self) {
            // SAFETY: the job handle is open until drop.
            unsafe { TerminateJobObject(self.0, 1) };
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: closed exactly once; with KILL_ON_JOB_CLOSE this also
            // ends any process still in the job.
            unsafe { CloseHandle(self.0) };
        }
    }
}

/// The first line a program prints for `args` (`--version`), or `None` when
/// it does not start, fails or takes longer than a few seconds.
pub(crate) async fn first_line(program: &Path, args: &[&str]) -> Option<String> {
    let mut command = command(program);
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let run = command.output();
    let output = tokio::time::timeout(std::time::Duration::from_secs(15), run)
        .await
        .ok()?
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .map(|l| l.chars().take(200).collect())
}
