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

/// Wakes the computer from sleep at a set moment: the schedule's start time.
///
/// A waitable timer that resumes the system belongs to the process that set
/// it, so Downpipe has to keep running (the tray is enough). Only Windows
/// lets a program do this without administrator rights; elsewhere `set`
/// does nothing.
pub struct WakeTimer {
    /// The timer's handle as a number, so the background loop stays `Send`.
    #[cfg(windows)]
    handle: isize,
    armed: Option<std::time::SystemTime>,
}

impl WakeTimer {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            handle: 0,
            armed: None,
        }
    }

    /// Wake the computer at `at`, or at no time. Setting the same moment
    /// again does nothing.
    pub fn set(&mut self, at: Option<std::time::SystemTime>) {
        if self.armed == at {
            return;
        }
        self.armed = at;
        #[cfg(windows)]
        self.arm_windows(at);
    }

    #[cfg(windows)]
    fn arm_windows(&mut self, at: Option<std::time::SystemTime>) {
        use windows_sys::Win32::Foundation::{ERROR_NOT_SUPPORTED, GetLastError};
        use windows_sys::Win32::System::Threading::{
            CancelWaitableTimer, CreateWaitableTimerW, SetWaitableTimer,
        };
        if self.handle == 0 {
            // SAFETY: no security attributes and no name: a private,
            // auto-reset timer this struct owns until it is dropped.
            let handle = unsafe { CreateWaitableTimerW(std::ptr::null(), 0, std::ptr::null()) };
            if handle.is_null() {
                tracing::warn!(
                    "could not create a wake timer: {}",
                    io::Error::last_os_error()
                );
                return;
            }
            self.handle = handle as isize;
        }
        let handle = self.handle as windows_sys::Win32::Foundation::HANDLE;
        let Some(at) = at else {
            // SAFETY: a valid timer handle owned by this struct.
            unsafe { CancelWaitableTimer(handle) };
            return;
        };
        // An absolute due time: 100 ns steps since 1601-01-01, UTC.
        let since_1970 = at.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let due = (since_1970.as_secs() as i64 + 11_644_473_600) * 10_000_000
            + i64::from(since_1970.subsec_nanos() / 100);
        // SAFETY: a valid timer handle; `due` outlives the call; no
        // completion routine; `fResume` asks the system to wake for it.
        let ok = unsafe { SetWaitableTimer(handle, &due, 0, None, std::ptr::null(), 1) };
        if ok == 0 {
            tracing::warn!(
                "could not set the wake timer: {}",
                io::Error::last_os_error()
            );
        // SAFETY: reads this thread's last error, set by the call above.
        } else if unsafe { GetLastError() } == ERROR_NOT_SUPPORTED {
            tracing::warn!("this computer cannot be woken by a timer");
        }
    }
}

impl Default for WakeTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WakeTimer {
    fn drop(&mut self) {
        #[cfg(windows)]
        if self.handle != 0 {
            // SAFETY: the handle was created by this struct and is closed once.
            unsafe {
                windows_sys::Win32::Foundation::CloseHandle(
                    self.handle as windows_sys::Win32::Foundation::HANDLE,
                )
            };
        }
    }
}

/// Whether Windows' power plan lets programs wake the computer while it is
/// plugged in ("Allow wake timers" set to Enable). `None` where that cannot
/// be read, and on other systems, where Downpipe does not wake it at all.
pub fn wake_timers_allowed() -> Option<bool> {
    #[cfg(windows)]
    {
        use windows_sys::Win32::Foundation::{ERROR_SUCCESS, LocalFree};
        use windows_sys::Win32::System::Power::{PowerGetActiveScheme, PowerReadACValueIndex};
        use windows_sys::core::GUID;
        // Sleep, and "Allow wake timers" in it (0 off, 1 on, 2 important only).
        const SUB_SLEEP: GUID = GUID::from_u128(0x238c9fa8_0aad_41ed_83f4_97be242c8f20);
        const RTC_WAKE: GUID = GUID::from_u128(0xbd3b718a_0680_4d9d_8ab2_e1d2b4ac806d);
        let mut scheme: *mut GUID = std::ptr::null_mut();
        // SAFETY: the out pointer is valid; the system allocates the GUID.
        if unsafe { PowerGetActiveScheme(std::ptr::null_mut(), &mut scheme) } != ERROR_SUCCESS
            || scheme.is_null()
        {
            return None;
        }
        let mut value = 0u32;
        // SAFETY: `scheme` came from the call above; the GUIDs and the out
        // value live through the call.
        let read = unsafe {
            PowerReadACValueIndex(
                std::ptr::null_mut(),
                scheme,
                &SUB_SLEEP,
                &RTC_WAKE,
                &mut value,
            )
        };
        // SAFETY: PowerGetActiveScheme's GUID is freed with LocalFree, once.
        unsafe { LocalFree(scheme.cast()) };
        (read == ERROR_SUCCESS).then_some(value == 1)
    }
    #[cfg(not(windows))]
    None
}
