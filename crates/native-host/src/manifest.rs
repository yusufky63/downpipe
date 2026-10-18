//! Native Messaging host manifests and where each browser looks for them.
//!
//! A browser starts `downpipe-native-host` only for extensions the manifest
//! names: `allowed_origins` (`chrome-extension://<id>/`, wildcards are not
//! accepted) for Chromium browsers and `allowed_extensions` for Firefox.

use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// Must match `HOST_NAME` in the extension.
pub const HOST_NAME: &str = "dev.downpipe.native_host";
/// Id of the extension in `apps/browser-extension`. The public `key` in its
/// Chrome manifest pins it, so unpacked builds in Chrome and Edge get it too.
pub const CHROMIUM_EXTENSION_ID: &str = "efgicfboaldflckpncfdojkeoaamflpf";
/// `browser_specific_settings.gecko.id` in the Firefox manifest.
pub const FIREFOX_EXTENSION_ID: &str = "downpipe@downpipe.dev";
/// File name of the host executable.
pub const HOST_EXE: &str = if cfg!(windows) {
    "downpipe-native-host.exe"
} else {
    "downpipe-native-host"
};

/// The host the desktop app ships: installers put Tauri sidecars next to the
/// app's own executable on every platform, and development builds have
/// Cargo's build of the host in the same folder.
pub fn host_beside(app_exe: &Path) -> PathBuf {
    app_exe.with_file_name(HOST_EXE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum Browser {
    Chrome,
    Edge,
    Chromium,
    Firefox,
}

impl Browser {
    pub const ALL: [Self; 4] = [Self::Chrome, Self::Edge, Self::Chromium, Self::Firefox];

    pub fn name(self) -> &'static str {
        match self {
            Self::Chrome => "chrome",
            Self::Edge => "edge",
            Self::Chromium => "chromium",
            Self::Firefox => "firefox",
        }
    }

    pub fn is_firefox(self) -> bool {
        self == Self::Firefox
    }

    /// The Downpipe extension's id in this browser.
    pub fn extension_id(self) -> &'static str {
        if self.is_firefox() {
            FIREFOX_EXTENSION_ID
        } else {
            CHROMIUM_EXTENSION_ID
        }
    }

    /// Key under `HKEY_CURRENT_USER` whose default value is the manifest path.
    pub fn registry_key(self) -> String {
        let vendor = match self {
            Self::Chrome => r"Software\Google\Chrome",
            Self::Edge => r"Software\Microsoft\Edge",
            Self::Chromium => r"Software\Chromium",
            Self::Firefox => r"Software\Mozilla",
        };
        format!(r"{vendor}\NativeMessagingHosts\{HOST_NAME}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOs,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else {
            Self::Linux
        }
    }
}

/// The per-user folders manifests go in. Separate from the real environment
/// so tests can point it at a temporary directory.
#[derive(Debug, Clone)]
pub struct Layout {
    pub platform: Platform,
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME` (usually `~/.config`); used on Linux.
    pub config: PathBuf,
    /// `%LOCALAPPDATA%`; used on Windows.
    pub data_local: PathBuf,
}

impl Layout {
    pub fn current() -> io::Result<Self> {
        let missing =
            |what: &str| io::Error::new(io::ErrorKind::NotFound, format!("no {what} folder"));
        let home = dirs::home_dir().ok_or_else(|| missing("home"))?;
        Ok(Self {
            platform: Platform::current(),
            config: dirs::config_dir().unwrap_or_else(|| home.join(".config")),
            data_local: dirs::data_local_dir().ok_or_else(|| missing("local app data"))?,
            home,
        })
    }

    /// Where the manifest for `browser` is written.
    pub fn manifest_path(&self, browser: Browser) -> PathBuf {
        let file = format!("{HOST_NAME}.json");
        match self.platform {
            // Any folder works on Windows; the registry points the browser at it.
            Platform::Windows => self
                .data_local
                .join("Downpipe")
                .join("NativeMessagingHosts")
                .join(browser.name())
                .join(file),
            Platform::MacOs => {
                let support = self.home.join("Library").join("Application Support");
                let vendor = match browser {
                    Browser::Chrome => support.join("Google").join("Chrome"),
                    Browser::Edge => support.join("Microsoft Edge"),
                    Browser::Chromium => support.join("Chromium"),
                    Browser::Firefox => support.join("Mozilla"),
                };
                vendor.join("NativeMessagingHosts").join(file)
            }
            Platform::Linux => match browser {
                Browser::Chrome => self.config.join("google-chrome"),
                Browser::Edge => self.config.join("microsoft-edge"),
                Browser::Chromium => self.config.join("chromium"),
                Browser::Firefox => self.home.join(".mozilla"),
            }
            .join(if browser.is_firefox() {
                "native-messaging-hosts"
            } else {
                "NativeMessagingHosts"
            })
            .join(file),
        }
    }

    /// Whether the browser seems to be installed for this user. On macOS and
    /// Linux that is one of its folders; registry keys are cheap, so Windows
    /// registers every browser.
    pub fn browser_present(&self, browser: Browser) -> bool {
        match self.platform {
            Platform::Windows => true,
            _ => self.browser_dirs(browser).iter().any(|dir| dir.is_dir()),
        }
    }

    /// Folders that exist once the browser has run for this user.
    fn browser_dirs(&self, browser: Browser) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = self
            .manifest_path(browser)
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .into_iter()
            .collect();
        if browser.is_firefox() {
            match self.platform {
                // Firefox keeps its profiles in `Firefox`; `Mozilla`, where
                // it looks for host manifests, may not exist yet.
                Platform::MacOs => dirs.push(
                    self.home
                        .join("Library")
                        .join("Application Support")
                        .join("Firefox"),
                ),
                // Ubuntu's Firefox snap keeps its profiles under ~/snap and
                // reads host manifests from ~/.mozilla through the desktop
                // portal, so ~/.mozilla may never have been created.
                Platform::Linux => dirs.push(self.home.join("snap").join("firefox")),
                Platform::Windows => {}
            }
        }
        dirs
    }
}

/// Where an AppImage's copy of the host goes (see [`copy_host`]): the desktop
/// app's data folder, `$XDG_DATA_HOME/dev.downpipe.desktop/native-host`.
pub fn appimage_host_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|dir| dir.join("dev.downpipe.desktop").join("native-host"))
}

/// Copy the host to `dir` unless an identical copy is already there, and
/// return the copy's path. An AppImage runs from a mount point that changes
/// with every start and is gone once the app quits, so browsers must be
/// pointed at a copy outside it; the host still answers `app_not_running`
/// from there.
pub fn copy_host(from: &Path, dir: &Path) -> io::Result<PathBuf> {
    let to = dir.join(HOST_EXE);
    let bytes = std::fs::read(from)?;
    if std::fs::read(&to).is_ok_and(|current| current == bytes) {
        return Ok(to);
    }
    std::fs::create_dir_all(dir)?;
    // A browser may be running the old copy; renaming over it leaves that
    // process its file, where writing into it would fail or corrupt it.
    let staged = dir.join(format!(".{HOST_EXE}.new"));
    std::fs::write(&staged, &bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
    }
    std::fs::rename(&staged, &to)?;
    Ok(to)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("`{0}` is not a Chrome/Edge extension id (32 letters a-p)")]
    ChromiumId(String),
    #[error("`{0}` is not a Firefox add-on id (name@domain or a {{GUID}})")]
    FirefoxId(String),
    #[error("at least one extension id is required")]
    NoIds,
    #[error("the host path must be absolute: {0}")]
    RelativePath(PathBuf),
}

/// The manifest JSON for one browser.
pub fn manifest(
    browser: Browser,
    host_path: &Path,
    ids: &[String],
) -> Result<Value, ManifestError> {
    if !host_path.is_absolute() {
        return Err(ManifestError::RelativePath(host_path.to_owned()));
    }
    if ids.is_empty() {
        return Err(ManifestError::NoIds);
    }
    let mut value = json!({
        "name": HOST_NAME,
        "description": "Downpipe browser bridge",
        "path": host_path,
        "type": "stdio",
    });
    if browser.is_firefox() {
        if let Some(bad) = ids.iter().find(|id| !is_firefox_id(id)) {
            return Err(ManifestError::FirefoxId(bad.clone()));
        }
        value["allowed_extensions"] = json!(ids);
    } else {
        if let Some(bad) = ids.iter().find(|id| !is_chromium_id(id)) {
            return Err(ManifestError::ChromiumId(bad.clone()));
        }
        let origins: Vec<String> = ids
            .iter()
            .map(|id| format!("chrome-extension://{id}/"))
            .collect();
        value["allowed_origins"] = json!(origins);
    }
    Ok(value)
}

/// Chrome derives ids from a SHA-256 of the public key, written with the
/// letters a-p instead of hex digits.
pub fn is_chromium_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

pub fn is_firefox_id(id: &str) -> bool {
    if id.len() > 80 {
        return false;
    }
    if let Some(guid) = id.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let groups: Vec<&str> = guid.split('-').collect();
        return groups.iter().map(|g| g.len()).eq([8, 4, 4, 4, 12])
            && groups
                .iter()
                .all(|g| g.bytes().all(|b| b.is_ascii_hexdigit()));
    }
    let allowed = |b: u8| b.is_ascii_alphanumeric() || b"-._".contains(&b);
    match id.split_once('@') {
        Some((name, domain)) => {
            !domain.is_empty() && name.bytes().all(allowed) && domain.bytes().all(allowed)
        }
        None => false,
    }
}

/// Write the manifest file and return its path.
pub fn write_manifest(layout: &Layout, browser: Browser, manifest: &Value) -> io::Result<PathBuf> {
    let path = layout.manifest_path(browser);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(manifest).map_err(io::Error::other)?;
    text.push('\n');
    std::fs::write(&path, text)?;
    Ok(path)
}

#[derive(Debug, Clone)]
pub struct Registration {
    pub browser: Browser,
    pub manifest_path: PathBuf,
    /// Windows only.
    pub registry_key: Option<String>,
}

/// Write the manifest and, on Windows, point the browser's registry key at it.
pub fn install(layout: &Layout, browser: Browser, manifest: &Value) -> io::Result<Registration> {
    let manifest_path = write_manifest(layout, browser, manifest)?;
    let registry_key = match layout.platform {
        Platform::Windows => {
            let key = browser.registry_key();
            registry::set_default(&key, &manifest_path)?;
            Some(key)
        }
        _ => None,
    };
    Ok(Registration {
        browser,
        manifest_path,
        registry_key,
    })
}

/// Remove the manifest and the registry key. Returns whether anything was there.
pub fn uninstall(layout: &Layout, browser: Browser) -> io::Result<bool> {
    let path = layout.manifest_path(browser);
    let mut removed = remove_file_if_present(&path)?;
    if layout.platform == Platform::Windows {
        removed |= registry::delete(&browser.registry_key())?;
    }
    remove_own_folders(layout, browser);
    Ok(removed)
}

/// Whose registration a browser has, from the point of view of one copy of
/// the host that is being removed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    /// Nothing is registered.
    Nobody,
    /// It starts the host in question.
    This,
    /// It starts another copy of the host, at this path.
    Other(PathBuf),
    /// The registered manifest is missing, not JSON, or not a host manifest,
    /// so it cannot be told which copy it was for.
    Unknown,
}

/// Who `found` belongs to, compared with `host`. Only a registration that
/// starts `host` is that host's to remove; one that starts another copy (a
/// build from source, another install) belongs to that copy.
pub fn owner(platform: Platform, browser: Browser, host: &Path, found: &Found) -> Owner {
    let contents = match found {
        Found::Nothing => return Owner::Nobody,
        Found::Manifest { contents, .. } => contents.as_ref(),
    };
    match contents.and_then(|v| parse_registered(browser, v)) {
        None => Owner::Unknown,
        Some((registered, _)) if same_path(platform, &registered, host) => Owner::This,
        Some((registered, _)) => Owner::Other(registered),
    }
}

/// Remove `browser`'s registration only if it starts `host`, and say whose it
/// was. This is what the desktop uninstallers run: a registration that another
/// copy of the host has taken over since must keep working.
pub fn uninstall_host(layout: &Layout, browser: Browser, host: &Path) -> io::Result<Owner> {
    let found = read_registration(layout, browser)?;
    let owner = match owner(layout.platform, browser, host, &found) {
        // The app writes the path it was started from, which can name the
        // same file differently (a junction, an 8.3 short name).
        Owner::Other(path) if same_file(&path, host) => Owner::This,
        owner => owner,
    };
    if let (Owner::This, Found::Manifest { path, .. }) = (&owner, &found) {
        remove_file_if_present(path)?;
        if layout.platform == Platform::Windows {
            registry::delete(&browser.registry_key())?;
        }
        remove_own_folders(layout, browser);
    }
    Ok(owner)
}

/// Both paths exist and are the same file.
fn same_file(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn remove_file_if_present(path: &Path) -> io::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// On Windows the manifests live in folders only Downpipe uses
/// (`%LOCALAPPDATA%\Downpipe\NativeMessagingHosts\<browser>`); drop them once
/// they are empty so an uninstall leaves nothing behind. On macOS and Linux
/// the folders belong to the browsers.
fn remove_own_folders(layout: &Layout, browser: Browser) {
    if layout.platform == Platform::Windows
        && let Some(dir) = layout.manifest_path(browser).parent()
    {
        remove_empty_dirs(dir, &layout.data_local.join("Downpipe"));
    }
}

/// Remove `dir` and then its parents while they are empty, up to and
/// including `root`. Stops at the first folder that still has something in it.
fn remove_empty_dirs(dir: &Path, root: &Path) {
    let mut next = Some(dir);
    while let Some(dir) = next.filter(|d| d.starts_with(root)) {
        if std::fs::remove_dir(dir).is_err() {
            break;
        }
        next = dir.parent();
    }
}

/// The manifest path the browser will actually use, if registered.
pub fn registered_path(layout: &Layout, browser: Browser) -> io::Result<Option<PathBuf>> {
    match layout.platform {
        Platform::Windows => Ok(registry::get_default(&browser.registry_key())?.map(PathBuf::from)),
        _ => {
            let path = layout.manifest_path(browser);
            Ok(path.is_file().then_some(path))
        }
    }
}

/// What a browser would load for the host right now, from [`read_registration`].
#[derive(Debug, Clone, PartialEq)]
pub enum Found {
    Nothing,
    /// The registered manifest and its JSON, `None` when the file is missing
    /// or is not JSON.
    Manifest {
        path: PathBuf,
        contents: Option<Value>,
    },
}

pub fn read_registration(layout: &Layout, browser: Browser) -> io::Result<Found> {
    let Some(path) = registered_path(layout, browser)? else {
        return Ok(Found::Nothing);
    };
    let contents = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok());
    Ok(Found::Manifest { path, contents })
}

/// Why a registration does not start the expected host for the extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stale {
    /// Nothing is registered.
    Missing,
    /// The registered file is missing, not JSON, or not a stdio host manifest
    /// named [`HOST_NAME`].
    Invalid,
    /// It starts another copy of the host, at this path.
    OtherHost(PathBuf),
    /// It starts the expected host but does not allow the Downpipe extension.
    ExtensionNotAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Plan {
    Keep,
    /// Write the manifest for the expected host, allowing `ids`.
    Write {
        why: Stale,
        ids: Vec<String>,
    },
}

/// Decide whether `browser`'s registration has to be rewritten so that it
/// starts `host` for the Downpipe extension. Other extension ids already
/// allowed (a store build, a developer's unpacked copy) are kept.
pub fn plan(platform: Platform, browser: Browser, host: &Path, found: &Found) -> Plan {
    let ours = browser.extension_id();
    let write = |why, ids: Vec<String>| Plan::Write { why, ids };
    let contents = match found {
        Found::Nothing => return write(Stale::Missing, vec![ours.into()]),
        Found::Manifest { contents, .. } => contents.as_ref(),
    };
    let Some((registered_host, mut ids)) = contents.and_then(|v| parse_registered(browser, v))
    else {
        return write(Stale::Invalid, vec![ours.into()]);
    };
    let allowed = ids.iter().any(|id| id == ours);
    if !allowed {
        ids.insert(0, ours.into());
    }
    if !same_path(platform, &registered_host, host) {
        write(Stale::OtherHost(registered_host), ids)
    } else if !allowed {
        write(Stale::ExtensionNotAllowed, ids)
    } else {
        Plan::Keep
    }
}

/// The host path and the valid extension ids of a manifest, or `None` if a
/// browser would not use it for this host.
fn parse_registered(browser: Browser, value: &Value) -> Option<(PathBuf, Vec<String>)> {
    if value["name"] != HOST_NAME || value["type"] != "stdio" {
        return None;
    }
    let host = value["path"].as_str().filter(|p| !p.is_empty())?;
    let list = if browser.is_firefox() {
        "allowed_extensions"
    } else {
        "allowed_origins"
    };
    let mut ids: Vec<String> = Vec::new();
    for entry in value[list].as_array()?.iter().filter_map(Value::as_str) {
        let id = if browser.is_firefox() {
            Some(entry).filter(|id| is_firefox_id(id))
        } else {
            entry
                .strip_prefix("chrome-extension://")
                .and_then(|rest| rest.strip_suffix('/'))
                .filter(|id| is_chromium_id(id))
        };
        if let Some(id) = id.filter(|id| !ids.iter().any(|known| known == id)) {
            ids.push(id.to_string());
        }
    }
    Some((PathBuf::from(host), ids))
}

/// Windows paths compare without regard to case or slash direction.
fn same_path(platform: Platform, a: &Path, b: &Path) -> bool {
    match platform {
        Platform::Windows => {
            let norm = |p: &Path| p.to_string_lossy().replace('/', "\\").to_lowercase();
            norm(a) == norm(b)
        }
        _ => a == b,
    }
}

/// One browser's registration compared with the expected host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// No profile folder for this user (macOS and Linux only).
    NotInstalled,
    /// Starts the expected host for the Downpipe extension.
    Current,
    /// Needs rewriting; [`check`] leaves it as it is.
    Stale(Stale),
    /// [`sync`] rewrote it.
    Updated(Stale),
    /// Reading the registration or writing the manifest failed.
    Failed(String),
}

/// Compare every browser's registration with `host` without changing anything.
pub fn check(layout: &Layout, host: &Path) -> Vec<(Browser, Status)> {
    Browser::ALL
        .into_iter()
        .map(|browser| (browser, status(layout, browser, host, false)))
        .collect()
}

/// Make every installed browser start `host` for the Downpipe extension, for
/// the current user only. Each browser is handled on its own, so one broken
/// registration does not keep the others from being fixed.
pub fn sync(layout: &Layout, host: &Path) -> Vec<(Browser, Status)> {
    Browser::ALL
        .into_iter()
        .map(|browser| (browser, status(layout, browser, host, true)))
        .collect()
}

fn status(layout: &Layout, browser: Browser, host: &Path, write: bool) -> Status {
    if !layout.browser_present(browser) {
        return Status::NotInstalled;
    }
    let found = match read_registration(layout, browser) {
        Ok(found) => found,
        Err(e) => return Status::Failed(e.to_string()),
    };
    match plan(layout.platform, browser, host, &found) {
        Plan::Keep => Status::Current,
        Plan::Write { why, .. } if !write => Status::Stale(why),
        Plan::Write { why, ids } => {
            let written = manifest(browser, host, &ids)
                .map_err(io::Error::other)
                .and_then(|value| install(layout, browser, &value));
            match written {
                Ok(_) => Status::Updated(why),
                Err(e) => Status::Failed(e.to_string()),
            }
        }
    }
}

/// `reg.exe` with argument arrays: no shell, no quoting surprises, and no
/// unsafe registry calls in this crate.
#[cfg(windows)]
mod registry {
    use std::io;
    use std::os::windows::process::CommandExt;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};

    /// The desktop app has no console, so without this flag every `reg.exe`
    /// it starts would flash a console window.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn reg(args: &[&str]) -> io::Result<Output> {
        let exe = std::env::var_os("SystemRoot")
            .map(|root| PathBuf::from(root).join("System32").join("reg.exe"))
            .unwrap_or_else(|| PathBuf::from("reg.exe"));
        Command::new(exe)
            .args(args)
            .stdin(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output()
    }

    fn hkcu(key: &str) -> String {
        format!(r"HKCU\{key}")
    }

    pub fn set_default(key: &str, value: &Path) -> io::Result<()> {
        let value = value.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "the manifest path is not valid UTF-8",
            )
        })?;
        let out = reg(&["add", &hkcu(key), "/ve", "/t", "REG_SZ", "/d", value, "/f"])?;
        if out.status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "reg add {} failed: {}",
                hkcu(key),
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    }

    pub fn get_default(key: &str) -> io::Result<Option<String>> {
        let out = reg(&["query", &hkcu(key), "/ve"])?;
        if !out.status.success() {
            return Ok(None);
        }
        // `    (Default)    REG_SZ    C:\path\to\manifest.json`
        let text = String::from_utf8_lossy(&out.stdout);
        Ok(text
            .lines()
            .find_map(|line| line.split_once("REG_SZ").map(|(_, v)| v.trim().to_string()))
            .filter(|v| !v.is_empty()))
    }

    pub fn delete(key: &str) -> io::Result<bool> {
        if !reg(&["query", &hkcu(key)])?.status.success() {
            return Ok(false);
        }
        let out = reg(&["delete", &hkcu(key), "/f"])?;
        if out.status.success() {
            Ok(true)
        } else {
            Err(io::Error::other(format!(
                "reg delete {} failed: {}",
                hkcu(key),
                String::from_utf8_lossy(&out.stderr).trim()
            )))
        }
    }
}

#[cfg(not(windows))]
mod registry {
    use std::io;
    use std::path::Path;

    fn unsupported() -> io::Error {
        io::Error::new(
            io::ErrorKind::Unsupported,
            "the registry exists only on Windows",
        )
    }

    pub fn set_default(_key: &str, _value: &Path) -> io::Result<()> {
        Err(unsupported())
    }

    pub fn get_default(_key: &str) -> io::Result<Option<String>> {
        Err(unsupported())
    }

    pub fn delete(_key: &str) -> io::Result<bool> {
        Err(unsupported())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(r"C:\Program Files\Downpipe\downpipe-native-host.exe")
        } else {
            PathBuf::from("/opt/downpipe/downpipe-native-host")
        }
    }

    fn layout(platform: Platform, root: &Path) -> Layout {
        Layout {
            platform,
            home: root.join("home"),
            config: root.join("home").join(".config"),
            data_local: root.join("local"),
        }
    }

    #[test]
    fn chromium_manifest_lists_exact_origins() {
        let ids = vec![CHROMIUM_EXTENSION_ID.to_string()];
        let m = manifest(Browser::Edge, &host(), &ids).unwrap();
        assert_eq!(m["name"], HOST_NAME);
        assert_eq!(m["type"], "stdio");
        assert_eq!(m["path"], host().to_str().unwrap());
        assert_eq!(
            m["allowed_origins"],
            json!([format!("chrome-extension://{CHROMIUM_EXTENSION_ID}/")])
        );
        assert!(m.get("allowed_extensions").is_none());
    }

    #[test]
    fn firefox_manifest_lists_add_on_ids() {
        let ids = vec![FIREFOX_EXTENSION_ID.to_string()];
        let m = manifest(Browser::Firefox, &host(), &ids).unwrap();
        assert_eq!(m["allowed_extensions"], json!([FIREFOX_EXTENSION_ID]));
        assert!(m.get("allowed_origins").is_none());
    }

    #[test]
    fn ids_and_paths_are_validated() {
        let one = |id: &str| vec![id.to_string()];
        for bad in [
            "*",
            "chrome-extension://x/",
            "ABCDEFGHIJKLMNOPABCDEFGHIJKLMNOP",
            "abc",
        ] {
            assert_eq!(
                manifest(Browser::Chrome, &host(), &one(bad)),
                Err(ManifestError::ChromiumId(bad.into()))
            );
        }
        for bad in ["*", "no-at-sign", "a@", "{1234}", "x@y z"] {
            assert_eq!(
                manifest(Browser::Firefox, &host(), &one(bad)),
                Err(ManifestError::FirefoxId(bad.into()))
            );
        }
        assert!(is_firefox_id("{b6b2b3a4-2c3d-4e5f-8a9b-0c1d2e3f4a5b}"));
        assert_eq!(
            manifest(Browser::Chrome, &host(), &[]),
            Err(ManifestError::NoIds)
        );
        assert_eq!(
            manifest(
                Browser::Chrome,
                Path::new("host.exe"),
                &one(CHROMIUM_EXTENSION_ID)
            ),
            Err(ManifestError::RelativePath("host.exe".into()))
        );
    }

    #[test]
    fn registry_keys_per_browser() {
        assert_eq!(
            Browser::Chrome.registry_key(),
            r"Software\Google\Chrome\NativeMessagingHosts\dev.downpipe.native_host"
        );
        assert_eq!(
            Browser::Edge.registry_key(),
            r"Software\Microsoft\Edge\NativeMessagingHosts\dev.downpipe.native_host"
        );
        assert_eq!(
            Browser::Firefox.registry_key(),
            r"Software\Mozilla\NativeMessagingHosts\dev.downpipe.native_host"
        );
    }

    #[test]
    fn manifest_locations_follow_each_browser() {
        let root = Path::new("/r");
        let mac = layout(Platform::MacOs, root);
        assert_eq!(
            mac.manifest_path(Browser::Chrome),
            Path::new(
                "/r/home/Library/Application Support/Google/Chrome/NativeMessagingHosts/dev.downpipe.native_host.json"
            )
        );
        assert_eq!(
            mac.manifest_path(Browser::Edge),
            Path::new(
                "/r/home/Library/Application Support/Microsoft Edge/NativeMessagingHosts/dev.downpipe.native_host.json"
            )
        );
        assert_eq!(
            mac.manifest_path(Browser::Firefox),
            Path::new(
                "/r/home/Library/Application Support/Mozilla/NativeMessagingHosts/dev.downpipe.native_host.json"
            )
        );
        let linux = layout(Platform::Linux, root);
        assert_eq!(
            linux.manifest_path(Browser::Chromium),
            Path::new(
                "/r/home/.config/chromium/NativeMessagingHosts/dev.downpipe.native_host.json"
            )
        );
        assert_eq!(
            linux.manifest_path(Browser::Firefox),
            Path::new("/r/home/.mozilla/native-messaging-hosts/dev.downpipe.native_host.json")
        );
        let windows = layout(Platform::Windows, root);
        assert_eq!(
            windows.manifest_path(Browser::Edge),
            Path::new("/r/local/Downpipe/NativeMessagingHosts/edge/dev.downpipe.native_host.json")
        );
    }

    #[test]
    fn writes_manifest_files_and_detects_browsers() {
        let tmp = tempfile::tempdir().unwrap();
        let linux = layout(Platform::Linux, tmp.path());
        assert!(!linux.browser_present(Browser::Chrome));
        std::fs::create_dir_all(linux.config.join("google-chrome")).unwrap();
        assert!(linux.browser_present(Browser::Chrome));
        assert!(!linux.browser_present(Browser::Edge));

        let m = manifest(Browser::Chrome, &host(), &[CHROMIUM_EXTENSION_ID.into()]).unwrap();
        let path = write_manifest(&linux, Browser::Chrome, &m).unwrap();
        let written: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written, m);
        assert_eq!(
            registered_path(&linux, Browser::Chrome).unwrap(),
            Some(path)
        );
        assert!(uninstall(&linux, Browser::Chrome).unwrap());
        assert!(!uninstall(&linux, Browser::Chrome).unwrap());
        assert_eq!(registered_path(&linux, Browser::Chrome).unwrap(), None);
    }

    #[test]
    fn firefox_counts_as_installed_from_its_profile_folders() {
        let tmp = tempfile::tempdir().unwrap();
        let linux = layout(Platform::Linux, tmp.path());
        assert!(!linux.browser_present(Browser::Firefox));
        std::fs::create_dir_all(linux.home.join("snap").join("firefox")).unwrap();
        assert!(linux.browser_present(Browser::Firefox));
        assert!(!linux.browser_present(Browser::Chrome));

        let mac = layout(Platform::MacOs, tmp.path());
        assert!(!mac.browser_present(Browser::Firefox));
        let support = mac.home.join("Library").join("Application Support");
        std::fs::create_dir_all(support.join("Firefox")).unwrap();
        assert!(mac.browser_present(Browser::Firefox));
        assert!(!mac.browser_present(Browser::Edge));
        std::fs::create_dir_all(support.join("Microsoft Edge")).unwrap();
        assert!(mac.browser_present(Browser::Edge));
    }

    #[test]
    fn host_copies_are_replaced_only_when_they_differ() {
        let tmp = tempfile::tempdir().unwrap();
        let shipped = tmp.path().join("mount").join(HOST_EXE);
        std::fs::create_dir_all(shipped.parent().unwrap()).unwrap();
        std::fs::write(&shipped, b"v1").unwrap();
        let dir = tmp.path().join("data").join("native-host");

        let copy = copy_host(&shipped, &dir).unwrap();
        assert_eq!(copy, dir.join(HOST_EXE));
        assert_eq!(std::fs::read(&copy).unwrap(), b"v1");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&copy).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
        let modified = std::fs::metadata(&copy).unwrap().modified().unwrap();
        assert_eq!(copy_host(&shipped, &dir).unwrap(), copy);
        assert_eq!(
            std::fs::metadata(&copy).unwrap().modified().unwrap(),
            modified,
            "an identical copy is left alone"
        );

        std::fs::write(&shipped, b"v2").unwrap();
        copy_host(&shipped, &dir).unwrap();
        assert_eq!(std::fs::read(&copy).unwrap(), b"v2");
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "no staged file left"
        );
        assert!(copy_host(&tmp.path().join("missing"), &dir).is_err());
    }

    #[test]
    fn bundled_host_sits_next_to_the_app() {
        let app = host().with_file_name("downpipe-desktop");
        assert_eq!(host_beside(&app), host().with_file_name(HOST_EXE));
        assert_eq!(host_beside(&app).parent(), app.parent());
    }

    fn registered(browser: Browser, host: &Path, ids: &[&str]) -> Found {
        let ids: Vec<String> = ids.iter().map(|id| id.to_string()).collect();
        Found::Manifest {
            path: PathBuf::from("manifest.json"),
            contents: Some(manifest(browser, host, &ids).unwrap()),
        }
    }

    const OTHER_ID: &str = "abcdefghijklmnopabcdefghijklmnop";

    #[test]
    fn plan_keeps_a_registration_for_this_host_and_extension() {
        let found = registered(Browser::Chrome, &host(), &[CHROMIUM_EXTENSION_ID, OTHER_ID]);
        assert_eq!(
            plan(Platform::current(), Browser::Chrome, &host(), &found),
            Plan::Keep
        );
        let found = registered(Browser::Firefox, &host(), &[FIREFOX_EXTENSION_ID]);
        assert_eq!(
            plan(Platform::current(), Browser::Firefox, &host(), &found),
            Plan::Keep
        );
    }

    #[test]
    fn plan_writes_missing_and_unusable_registrations() {
        let fresh = |why| Plan::Write {
            why,
            ids: vec![CHROMIUM_EXTENSION_ID.to_string()],
        };
        let platform = Platform::current();
        assert_eq!(
            plan(platform, Browser::Edge, &host(), &Found::Nothing),
            fresh(Stale::Missing)
        );
        let unreadable = Found::Manifest {
            path: PathBuf::from("gone.json"),
            contents: None,
        };
        assert_eq!(
            plan(platform, Browser::Edge, &host(), &unreadable),
            fresh(Stale::Invalid)
        );
        let mut value = manifest(Browser::Edge, &host(), &[CHROMIUM_EXTENSION_ID.into()]).unwrap();
        for (field, bad) in [
            ("name", json!("com.example.other")),
            ("type", json!("socket")),
            ("path", json!("")),
            ("allowed_origins", json!("not a list")),
        ] {
            let saved = value[field].take();
            value[field] = bad;
            let found = Found::Manifest {
                path: PathBuf::from("m.json"),
                contents: Some(value.clone()),
            };
            assert_eq!(
                plan(platform, Browser::Edge, &host(), &found),
                fresh(Stale::Invalid),
                "{field}"
            );
            value[field] = saved;
        }
    }

    #[test]
    fn plan_moves_the_registration_to_this_host_and_keeps_other_ids() {
        let old = host().with_file_name("old-host");
        let found = registered(Browser::Chrome, &old, &[OTHER_ID]);
        assert_eq!(
            plan(Platform::current(), Browser::Chrome, &host(), &found),
            Plan::Write {
                why: Stale::OtherHost(old.clone()),
                ids: vec![CHROMIUM_EXTENSION_ID.into(), OTHER_ID.into()],
            }
        );
        let found = registered(Browser::Chrome, &host(), &[OTHER_ID]);
        assert_eq!(
            plan(Platform::current(), Browser::Chrome, &host(), &found),
            Plan::Write {
                why: Stale::ExtensionNotAllowed,
                ids: vec![CHROMIUM_EXTENSION_ID.into(), OTHER_ID.into()],
            }
        );
        let found = registered(Browser::Firefox, &old, &[FIREFOX_EXTENSION_ID]);
        assert_eq!(
            plan(Platform::current(), Browser::Firefox, &host(), &found),
            Plan::Write {
                why: Stale::OtherHost(old),
                ids: vec![FIREFOX_EXTENSION_ID.into()],
            }
        );
    }

    #[test]
    fn plan_drops_ids_a_browser_would_reject() {
        let found = Found::Manifest {
            path: PathBuf::from("m.json"),
            contents: Some(json!({
                "name": HOST_NAME,
                "type": "stdio",
                "path": "/old/downpipe-native-host",
                "allowed_origins": [
                    "chrome-extension://*/",
                    format!("chrome-extension://{OTHER_ID}"),
                    format!("chrome-extension://{OTHER_ID}/"),
                    format!("chrome-extension://{OTHER_ID}/"),
                    7,
                ],
            })),
        };
        let Plan::Write { ids, .. } = plan(Platform::Linux, Browser::Chrome, &host(), &found)
        else {
            panic!("expected a rewrite");
        };
        assert_eq!(ids, [CHROMIUM_EXTENSION_ID, OTHER_ID]);
    }

    #[test]
    fn host_paths_compare_like_the_file_system() {
        let lower = Path::new(r"c:/users/me/appdata/local/downpipe/downpipe-native-host.exe");
        let upper = Path::new(r"C:\Users\Me\AppData\Local\Downpipe\downpipe-native-host.exe");
        // `manifest` refuses relative paths, and a drive letter means nothing
        // outside Windows.
        if cfg!(windows) {
            let found = registered(Browser::Edge, upper, &[CHROMIUM_EXTENSION_ID]);
            assert_eq!(
                plan(Platform::Windows, Browser::Edge, lower, &found),
                Plan::Keep
            );
        }
        assert!(same_path(Platform::Windows, lower, upper));
        assert!(!same_path(Platform::Linux, lower, upper));
        assert!(!same_path(Platform::MacOs, lower, upper));
    }

    #[test]
    fn sync_registers_installed_browsers_and_check_only_reports() {
        let tmp = tempfile::tempdir().unwrap();
        let linux = layout(Platform::Linux, tmp.path());
        std::fs::create_dir_all(linux.config.join("google-chrome")).unwrap();
        std::fs::create_dir_all(linux.home.join(".mozilla")).unwrap();
        let first = host();
        let moved = host().with_file_name("moved").join(HOST_EXE);
        let status_of = |list: &[(Browser, Status)], browser| {
            list.iter()
                .find(|(b, _)| *b == browser)
                .map(|(_, s)| s.clone())
                .unwrap()
        };

        let report = check(&linux, &first);
        assert_eq!(
            status_of(&report, Browser::Chrome),
            Status::Stale(Stale::Missing)
        );
        assert_eq!(status_of(&report, Browser::Edge), Status::NotInstalled);
        assert_eq!(
            registered_path(&linux, Browser::Chrome).unwrap(),
            None,
            "check must not write"
        );

        let report = sync(&linux, &first);
        assert_eq!(
            status_of(&report, Browser::Chrome),
            Status::Updated(Stale::Missing)
        );
        assert_eq!(
            status_of(&report, Browser::Firefox),
            Status::Updated(Stale::Missing)
        );
        assert_eq!(status_of(&report, Browser::Edge), Status::NotInstalled);
        assert_eq!(status_of(&report, Browser::Chromium), Status::NotInstalled);
        assert!(
            sync(&linux, &first)
                .iter()
                .all(|(_, s)| matches!(s, Status::Current | Status::NotInstalled))
        );

        // The app was moved or reinstalled elsewhere.
        assert_eq!(
            status_of(&check(&linux, &moved), Browser::Chrome),
            Status::Stale(Stale::OtherHost(first.clone()))
        );
        assert_eq!(
            status_of(&sync(&linux, &moved), Browser::Chrome),
            Status::Updated(Stale::OtherHost(first))
        );
        let path = linux.manifest_path(Browser::Chrome);
        let written: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(written["path"], moved.to_str().unwrap());

        std::fs::write(&path, "not json").unwrap();
        assert_eq!(
            status_of(&sync(&linux, &moved), Browser::Chrome),
            Status::Updated(Stale::Invalid)
        );
        assert_eq!(
            status_of(&check(&linux, &moved), Browser::Chrome),
            Status::Current
        );
    }

    #[test]
    fn owner_tells_this_host_from_other_copies() {
        let platform = Platform::current();
        assert_eq!(
            owner(platform, Browser::Chrome, &host(), &Found::Nothing),
            Owner::Nobody
        );
        let found = registered(Browser::Chrome, &host(), &[CHROMIUM_EXTENSION_ID]);
        assert_eq!(
            owner(platform, Browser::Chrome, &host(), &found),
            Owner::This
        );
        // Which extensions it allows does not change which host it starts.
        let found = registered(Browser::Edge, &host(), &[OTHER_ID]);
        assert_eq!(owner(platform, Browser::Edge, &host(), &found), Owner::This);
        let source_build = host().with_file_name("target").join(HOST_EXE);
        let found = registered(Browser::Firefox, &source_build, &[FIREFOX_EXTENSION_ID]);
        assert_eq!(
            owner(platform, Browser::Firefox, &host(), &found),
            Owner::Other(source_build)
        );
        let unreadable = Found::Manifest {
            path: PathBuf::from("gone.json"),
            contents: None,
        };
        assert_eq!(
            owner(platform, Browser::Chrome, &host(), &unreadable),
            Owner::Unknown
        );
        let foreign = Found::Manifest {
            path: PathBuf::from("m.json"),
            contents: Some(json!({ "name": "com.example.other", "type": "stdio", "path": "/x" })),
        };
        assert_eq!(
            owner(platform, Browser::Chrome, &host(), &foreign),
            Owner::Unknown
        );
        if cfg!(windows) {
            let upper = Path::new(r"C:\Users\Me\AppData\Local\Downpipe\downpipe-native-host.exe");
            let lower = Path::new(r"c:/users/me/appdata/local/downpipe/downpipe-native-host.exe");
            let found = registered(Browser::Edge, upper, &[CHROMIUM_EXTENSION_ID]);
            assert_eq!(
                owner(Platform::Windows, Browser::Edge, lower, &found),
                Owner::This
            );
        }
    }

    #[test]
    fn uninstall_host_removes_only_its_own_registration() {
        let tmp = tempfile::tempdir().unwrap();
        let linux = layout(Platform::Linux, tmp.path());
        std::fs::create_dir_all(linux.config.join("google-chrome")).unwrap();
        std::fs::create_dir_all(linux.home.join(".mozilla")).unwrap();
        let installed = host();
        let source_build = host().with_file_name("target").join(HOST_EXE);

        // A build from source took the registration over after the install.
        sync(&linux, &source_build);
        assert_eq!(
            uninstall_host(&linux, Browser::Chrome, &installed).unwrap(),
            Owner::Other(source_build.clone())
        );
        assert!(linux.manifest_path(Browser::Chrome).is_file());

        sync(&linux, &installed);
        assert_eq!(
            uninstall_host(&linux, Browser::Chrome, &installed).unwrap(),
            Owner::This
        );
        assert_eq!(registered_path(&linux, Browser::Chrome).unwrap(), None);
        assert!(linux.config.join("google-chrome").is_dir());
        assert_eq!(
            uninstall_host(&linux, Browser::Chrome, &installed).unwrap(),
            Owner::Nobody
        );
        assert_eq!(
            uninstall_host(&linux, Browser::Edge, &installed).unwrap(),
            Owner::Nobody
        );

        let firefox = linux.manifest_path(Browser::Firefox);
        std::fs::write(&firefox, "not json").unwrap();
        assert_eq!(
            uninstall_host(&linux, Browser::Firefox, &installed).unwrap(),
            Owner::Unknown
        );
        assert_eq!(std::fs::read_to_string(&firefox).unwrap(), "not json");
    }

    #[test]
    fn uninstall_host_matches_another_spelling_of_the_same_file() {
        let tmp = tempfile::tempdir().unwrap();
        let linux = layout(Platform::Linux, tmp.path());
        std::fs::create_dir_all(linux.config.join("google-chrome")).unwrap();
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(bin.join("sub")).unwrap();
        let installed = bin.join(HOST_EXE);
        std::fs::write(&installed, "").unwrap();
        let spelled = bin.join("sub").join("..").join(HOST_EXE);

        let value = manifest(Browser::Chrome, &spelled, &[CHROMIUM_EXTENSION_ID.into()]).unwrap();
        install(&linux, Browser::Chrome, &value).unwrap();
        let found = read_registration(&linux, Browser::Chrome).unwrap();
        assert_eq!(
            owner(Platform::Linux, Browser::Chrome, &installed, &found),
            Owner::Other(spelled)
        );
        assert_eq!(
            uninstall_host(&linux, Browser::Chrome, &installed).unwrap(),
            Owner::This
        );
        assert_eq!(registered_path(&linux, Browser::Chrome).unwrap(), None);
    }

    #[test]
    fn empty_manifest_folders_are_removed_up_to_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("local").join("Downpipe");
        let hosts = root.join("NativeMessagingHosts");
        let (chrome, edge) = (hosts.join("chrome"), hosts.join("edge"));
        std::fs::create_dir_all(&chrome).unwrap();
        std::fs::create_dir_all(&edge).unwrap();
        std::fs::write(edge.join("m.json"), "{}").unwrap();

        remove_empty_dirs(&chrome, &root);
        assert!(!chrome.exists());
        assert!(edge.join("m.json").is_file());

        std::fs::remove_file(edge.join("m.json")).unwrap();
        remove_empty_dirs(&edge, &root);
        assert!(!root.exists());
        assert!(tmp.path().join("local").is_dir(), "stops at the root");

        let outside = tmp.path().join("other");
        std::fs::create_dir_all(&outside).unwrap();
        remove_empty_dirs(&outside, &root);
        assert!(outside.is_dir());
    }
}
