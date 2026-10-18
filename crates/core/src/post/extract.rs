//! Extracting a finished archive into a new folder next to it.
//!
//! Archives come from servers and are untrusted. Every entry name is turned
//! into a relative path of sanitized components inside the new folder:
//! names with `..`, absolute paths, drive letters and UNC paths are left
//! out, symbolic and hard links, devices and FIFOs are left out, and files
//! are created with `create_new`, so nothing outside the folder is ever
//! written and nothing inside it is overwritten. The folder is new and holds
//! no links, so no path can lead out of it. Sizes are checked against the
//! bytes actually written (headers can lie): an archive may not expand past
//! [`Limits`], which stops zip bombs. Nested archives are not opened.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use downpipe_http::filename;

/// The archive formats Downpipe can open itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveKind {
    Zip,
    Tar,
    TarGz,
    TarXz,
    SevenZ,
}

/// Name endings and their kinds, longest first.
const ENDINGS: [(&str, ArchiveKind); 7] = [
    (".tar.gz", ArchiveKind::TarGz),
    (".tar.xz", ArchiveKind::TarXz),
    (".tgz", ArchiveKind::TarGz),
    (".txz", ArchiveKind::TarXz),
    (".tar", ArchiveKind::Tar),
    (".zip", ArchiveKind::Zip),
    (".7z", ArchiveKind::SevenZ),
];

impl ArchiveKind {
    /// The kind a file name says, if Downpipe can extract it.
    pub fn of(file_name: &str) -> Option<Self> {
        let lower = file_name.to_ascii_lowercase();
        ENDINGS
            .iter()
            .find(|(end, _)| lower.len() > end.len() && lower.ends_with(end))
            .map(|&(_, kind)| kind)
    }
}

/// The name without the archive ending, for the folder: `a.tar.gz` -> `a`.
pub fn stem(file_name: &str) -> &str {
    let lower = file_name.to_ascii_lowercase();
    ENDINGS
        .iter()
        .find(|(end, _)| lower.len() > end.len() && lower.ends_with(end))
        .map_or(file_name, |(end, _)| {
            &file_name[..file_name.len() - end.len()]
        })
}

/// How far an archive may expand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Bytes written in all, whatever the archive's size.
    pub max_total: u64,
    /// Bytes written per byte of archive, beyond `allowance`.
    pub max_ratio: u64,
    /// Bytes any archive may expand to before `max_ratio` applies, so small
    /// archives of text (which compress very well) still extract.
    pub allowance: u64,
    /// Files and folders.
    pub max_entries: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_total: 64 << 30,
            max_ratio: 200,
            allowance: 256 << 20,
            max_entries: 100_000,
        }
    }
}

impl Limits {
    fn budget(&self, archive_len: u64) -> u64 {
        archive_len
            .saturating_mul(self.max_ratio)
            .max(self.allowance)
            .min(self.max_total)
    }
}

/// A finished extraction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extracted {
    pub folder: PathBuf,
    pub files: u64,
    pub bytes: u64,
    /// Entries left out: unsafe names, links, special files, duplicates.
    pub skipped: u64,
}

/// Why nothing was extracted. The folder is removed again in every case.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("the archive is encrypted")]
    Encrypted,
    #[error("the archive expands to more than {0} bytes")]
    TooLarge(u64),
    #[error("the archive has more than {0} entries")]
    TooManyEntries(u64),
    #[error("the archive is damaged or uses an unsupported method: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("stopped")]
    Canceled,
}

impl ExtractError {
    /// Stable name the UI explains.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Encrypted => "encrypted",
            Self::TooLarge(_) => "too_large",
            Self::TooManyEntries(_) => "too_many_entries",
            Self::Corrupt(_) => "corrupt",
            Self::Io(_) => "io",
            Self::Canceled => "canceled",
        }
    }
}

/// Extract `archive` into a new folder next to it, named after it
/// (`a.zip` -> `a`, `a (1)` when that exists). Blocking; checks `cancel`
/// between entries and while writing.
pub fn extract(
    archive: &Path,
    kind: ArchiveKind,
    limits: Limits,
    cancel: &AtomicBool,
) -> Result<Extracted, ExtractError> {
    let parent = archive.parent().unwrap_or(Path::new("."));
    let name = archive
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let folder = new_folder(parent, &filename::sanitize(stem(&name)))?;
    let archive_len = fs::metadata(archive)?.len();
    let mut out = Sink {
        root: folder.clone(),
        budget: limits.budget(archive_len),
        max_entries: limits.max_entries,
        entries: 0,
        files: 0,
        bytes: 0,
        skipped: 0,
        cancel,
    };
    let result = match kind {
        ArchiveKind::Zip => zip_entries(archive, &mut out),
        ArchiveKind::Tar => tar_entries(File::open(archive)?, &mut out),
        ArchiveKind::TarGz => tar_entries(
            flate2::read::MultiGzDecoder::new(io::BufReader::new(File::open(archive)?)),
            &mut out,
        ),
        // The dictionary an XZ header asks for is allocated up front; a
        // crafted header must not take all the memory.
        ArchiveKind::TarXz => tar_entries(
            lzma_rust2::XzReader::new_mem_limit(
                io::BufReader::new(File::open(archive)?),
                true,
                XZ_MEMORY_KIB,
            ),
            &mut out,
        ),
        ArchiveKind::SevenZ => seven_z_entries(archive, &mut out),
    };
    match result {
        Ok(()) => Ok(Extracted {
            folder,
            files: out.files,
            bytes: out.bytes,
            skipped: out.skipped,
        }),
        Err(err) => {
            // The folder is Downpipe's own, made above; nothing else is in it.
            let _ = fs::remove_dir_all(&folder);
            Err(err)
        }
    }
}

/// 512 MiB, eight times what `xz -9` needs.
const XZ_MEMORY_KIB: u32 = 512 * 1024;

/// A folder that did not exist before, so the extraction owns all of it.
fn new_folder(parent: &Path, name: &str) -> io::Result<PathBuf> {
    for n in 0..1000 {
        let candidate = if n == 0 {
            parent.join(name)
        } else {
            parent.join(format!("{name} ({n})"))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "no free folder name next to the archive",
    ))
}

/// Where entries go, and what they may cost.
struct Sink<'a> {
    root: PathBuf,
    budget: u64,
    max_entries: u64,
    entries: u64,
    files: u64,
    bytes: u64,
    skipped: u64,
    cancel: &'a AtomicBool,
}

/// What an entry is, as far as extracting goes.
enum Kind {
    File {
        executable: bool,
    },
    Dir,
    /// Links, devices, FIFOs and the like: left out.
    Other,
}

impl Sink<'_> {
    fn check(&mut self) -> Result<(), ExtractError> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(ExtractError::Canceled);
        }
        self.entries += 1;
        if self.entries > self.max_entries {
            return Err(ExtractError::TooManyEntries(self.max_entries));
        }
        Ok(())
    }

    /// Write one entry. `declared` is the size the archive claims, checked
    /// before writing so an obvious bomb stops early.
    fn entry(
        &mut self,
        name: &str,
        kind: Kind,
        declared: u64,
        data: &mut dyn Read,
    ) -> Result<(), ExtractError> {
        self.check()?;
        let Some(relative) = safe_relative(name) else {
            self.skipped += 1;
            return Ok(());
        };
        let path = self.root.join(&relative);
        match kind {
            Kind::Other => {
                self.skipped += 1;
                Ok(())
            }
            Kind::Dir => {
                if fs::create_dir_all(&path).is_err() {
                    // A file of that name came first.
                    self.skipped += 1;
                }
                Ok(())
            }
            Kind::File { executable } => {
                if declared > self.budget.saturating_sub(self.bytes) {
                    return Err(ExtractError::TooLarge(self.budget));
                }
                if let Some(parent) = path.parent()
                    && fs::create_dir_all(parent).is_err()
                {
                    self.skipped += 1;
                    return Ok(());
                }
                let file = match OpenOptions::new().write(true).create_new(true).open(&path) {
                    Ok(file) => file,
                    // The same name twice, or a folder of that name.
                    Err(e)
                        if matches!(
                            e.kind(),
                            io::ErrorKind::AlreadyExists | io::ErrorKind::PermissionDenied
                        ) =>
                    {
                        self.skipped += 1;
                        return Ok(());
                    }
                    Err(e) => return Err(e.into()),
                };
                self.copy(data, file)?;
                self.files += 1;
                if executable {
                    mark_executable(&path);
                }
                Ok(())
            }
        }
    }

    fn copy(&mut self, data: &mut dyn Read, mut file: File) -> Result<(), ExtractError> {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                return Err(ExtractError::Canceled);
            }
            let n = match data.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(corrupt(e)),
            };
            self.bytes += n as u64;
            if self.bytes > self.budget {
                return Err(ExtractError::TooLarge(self.budget));
            }
            file.write_all(&buf[..n])?;
        }
        file.flush()?;
        Ok(())
    }
}

/// Read errors while decompressing mean a damaged archive; disk errors are
/// reported as they are.
fn corrupt(e: io::Error) -> ExtractError {
    match e.kind() {
        io::ErrorKind::InvalidData | io::ErrorKind::UnexpectedEof | io::ErrorKind::Other => {
            ExtractError::Corrupt(e.to_string())
        }
        _ => ExtractError::Io(e),
    }
}

/// An entry name as a path inside the folder, or `None` when it is not
/// safe: `..`, absolute paths, drive letters, UNC and device paths. Each
/// component is sanitized like a downloaded file's name, which also keeps
/// out Windows device names and `:` (alternate data streams).
pub fn safe_relative(name: &str) -> Option<PathBuf> {
    if name.contains('\0') || name.starts_with(['/', '\\']) {
        return None;
    }
    let mut out = PathBuf::new();
    for (i, part) in name.split(['/', '\\']).enumerate() {
        match part {
            "" | "." => {}
            ".." => return None,
            // `C:` or `C:x` as the first component is a drive.
            p if i == 0 && p.len() >= 2 && p.as_bytes()[1] == b':' => return None,
            p => out.push(filename::sanitize(p)),
        }
    }
    (!out.as_os_str().is_empty()).then_some(out)
}

#[cfg(unix)]
fn mark_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn mark_executable(_path: &Path) {}

/// Unix file type bits of a mode.
const S_IFMT: u32 = 0o170_000;
const S_IFREG: u32 = 0o100_000;
const S_IFDIR: u32 = 0o040_000;

fn zip_entries(archive: &Path, out: &mut Sink<'_>) -> Result<(), ExtractError> {
    let file = io::BufReader::new(File::open(archive)?);
    let mut zip = zip::ZipArchive::new(file).map_err(zip_error)?;
    for i in 0..zip.len() {
        let (name, kind, declared) = {
            let entry = zip.by_index_raw(i).map_err(zip_error)?;
            if entry.encrypted() {
                return Err(ExtractError::Encrypted);
            }
            let mode = entry.unix_mode();
            let kind = if entry.is_symlink() {
                Kind::Other
            } else if entry.is_dir() {
                Kind::Dir
            } else {
                match mode.map(|m| m & S_IFMT) {
                    None | Some(0) | Some(S_IFREG) => Kind::File {
                        executable: mode.is_some_and(|m| m & 0o111 != 0),
                    },
                    Some(S_IFDIR) => Kind::Dir,
                    Some(_) => Kind::Other,
                }
            };
            (
                String::from_utf8_lossy(entry.name_raw()).into_owned(),
                kind,
                entry.size(),
            )
        };
        let mut entry = zip.by_index(i).map_err(zip_error)?;
        out.entry(&name, kind, declared, &mut entry)?;
    }
    Ok(())
}

fn zip_error(e: zip::result::ZipError) -> ExtractError {
    use zip::result::ZipError;
    match e {
        ZipError::Io(e) => corrupt(e),
        ZipError::UnsupportedArchive(m) if m == ZipError::PASSWORD_REQUIRED => {
            ExtractError::Encrypted
        }
        other => ExtractError::Corrupt(other.to_string()),
    }
}

fn tar_entries(reader: impl Read, out: &mut Sink<'_>) -> Result<(), ExtractError> {
    use tar::EntryType;
    let mut archive = tar::Archive::new(reader);
    for entry in archive.entries().map_err(corrupt)? {
        let mut entry = entry.map_err(corrupt)?;
        let header = entry.header();
        let kind = match header.entry_type() {
            EntryType::Regular | EntryType::Continuous => Kind::File {
                executable: header.mode().is_ok_and(|m| m & 0o111 != 0),
            },
            EntryType::Directory => Kind::Dir,
            // Links (hard and symbolic), devices, FIFOs, sparse files and
            // headers this reader does not handle itself.
            _ => Kind::Other,
        };
        let declared = header.size().unwrap_or(0);
        // Long names (GNU and PAX) are already applied here.
        let name = String::from_utf8_lossy(&entry.path_bytes()).into_owned();
        out.entry(&name, kind, declared, &mut entry)?;
    }
    Ok(())
}

/// Windows attribute bits 7-Zip writes: a directory, a reparse point
/// (link), and "the high 16 bits are a Unix mode".
const ATTR_DIRECTORY: u32 = 0x10;
const ATTR_REPARSE_POINT: u32 = 0x400;
const ATTR_UNIX_EXTENSION: u32 = 0x8000;

fn seven_z_entries(archive: &Path, out: &mut Sink<'_>) -> Result<(), ExtractError> {
    use sevenz_rust2::{ArchiveReader, Error, Password};
    let mut reader = ArchiveReader::open(archive, Password::empty()).map_err(seven_z_error)?;
    let mut failure: Option<ExtractError> = None;
    let walked = reader.for_each_entries(|entry, data| {
        let attrs = if entry.has_windows_attributes {
            entry.windows_attributes
        } else {
            0
        };
        let unix = (attrs & ATTR_UNIX_EXTENSION != 0).then_some(attrs >> 16);
        let kind = if entry.is_anti_item || attrs & ATTR_REPARSE_POINT != 0 {
            Kind::Other
        } else if entry.is_directory || attrs & ATTR_DIRECTORY != 0 {
            Kind::Dir
        } else {
            match unix.map(|m| m & S_IFMT) {
                None | Some(0) | Some(S_IFREG) => Kind::File {
                    executable: unix.is_some_and(|m| m & 0o111 != 0),
                },
                Some(S_IFDIR) => Kind::Dir,
                Some(_) => Kind::Other,
            }
        };
        let is_file = matches!(kind, Kind::File { .. });
        match out.entry(&entry.name, kind, entry.size, data) {
            Ok(()) => {
                // In a solid block, skipping an entry still means reading
                // through it, and those bytes count against the budget too.
                if !is_file && let Err(e) = drain(data, out) {
                    failure = Some(e);
                    return Ok(false);
                }
                Ok(true)
            }
            Err(e) => {
                failure = Some(e);
                Ok(false)
            }
        }
    });
    if let Some(e) = failure {
        return Err(e);
    }
    walked.map_err(|e| match e {
        Error::PasswordRequired | Error::MaybeBadPassword(_) => ExtractError::Encrypted,
        other => seven_z_error(other),
    })
}

fn drain(data: &mut dyn Read, out: &mut Sink<'_>) -> Result<(), ExtractError> {
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = data.read(&mut buf).map_err(corrupt)?;
        if n == 0 {
            return Ok(());
        }
        out.bytes += n as u64;
        if out.bytes > out.budget {
            return Err(ExtractError::TooLarge(out.budget));
        }
    }
}

fn seven_z_error(e: sevenz_rust2::Error) -> ExtractError {
    use sevenz_rust2::Error;
    match e {
        Error::PasswordRequired | Error::MaybeBadPassword(_) => ExtractError::Encrypted,
        Error::Io(e, _) | Error::FileOpen(e, _) if e.kind() == io::ErrorKind::NotFound => {
            ExtractError::Io(e)
        }
        other => ExtractError::Corrupt(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_cancel() -> AtomicBool {
        AtomicBool::new(false)
    }

    fn files_in(dir: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in fs::read_dir(&d).unwrap() {
                let e = e.unwrap();
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    let rel = p
                        .strip_prefix(dir)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    out.push(rel);
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn kinds_and_stems() {
        assert_eq!(ArchiveKind::of("a.ZIP"), Some(ArchiveKind::Zip));
        assert_eq!(ArchiveKind::of("a.tar.gz"), Some(ArchiveKind::TarGz));
        assert_eq!(ArchiveKind::of("a.tgz"), Some(ArchiveKind::TarGz));
        assert_eq!(ArchiveKind::of("a.tar.xz"), Some(ArchiveKind::TarXz));
        assert_eq!(ArchiveKind::of("a.7z"), Some(ArchiveKind::SevenZ));
        assert_eq!(ArchiveKind::of("a.rar"), None);
        assert_eq!(ArchiveKind::of(".zip"), None);
        assert_eq!(
            ArchiveKind::of("a.gz"),
            None,
            "a single gzip file is not an archive"
        );
        assert_eq!(stem("Release.v2.tar.gz"), "Release.v2");
        assert_eq!(stem("photos.zip"), "photos");
    }

    #[test]
    fn unsafe_names_are_refused() {
        for name in [
            "../evil.txt",
            "a/../../evil.txt",
            "..\\evil.txt",
            "/etc/passwd",
            "\\Windows\\x.dll",
            "\\\\server\\share\\x",
            "C:\\Windows\\x.dll",
            "C:x.dll",
            "a\0b",
            "",
            "./",
        ] {
            assert_eq!(safe_relative(name), None, "{name:?}");
        }
        assert_eq!(
            safe_relative("./docs//read me.txt"),
            Some(Path::new("docs").join("read me.txt"))
        );
        assert_eq!(
            safe_relative("a/CON/b:c.txt"),
            Some(Path::new("a").join("_CON").join("b_c.txt"))
        );
    }

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        use zip::write::SimpleFileOptions;
        let mut w = zip::ZipWriter::new(File::create(path).unwrap());
        let opts =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            if name.ends_with('/') {
                w.add_directory(*name, opts).unwrap();
            } else {
                w.start_file(*name, opts).unwrap();
                w.write_all(data).unwrap();
            }
        }
        w.finish().unwrap();
    }

    #[test]
    fn zip_extracts_into_a_new_folder() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("pack.zip");
        write_zip(
            &archive,
            &[
                ("docs/", b""),
                ("docs/a.txt", b"hello"),
                ("b.bin", &[7u8; 1000]),
            ],
        );
        fs::create_dir(dir.path().join("pack")).unwrap();
        let done = extract(&archive, ArchiveKind::Zip, Limits::default(), &no_cancel()).unwrap();
        assert_eq!(
            done.folder,
            dir.path().join("pack (1)"),
            "the existing folder is kept"
        );
        assert_eq!(files_in(&done.folder), ["b.bin", "docs/a.txt"]);
        assert_eq!((done.files, done.bytes, done.skipped), (2, 1005, 0));
        assert_eq!(fs::read(done.folder.join("docs/a.txt")).unwrap(), b"hello");
    }

    #[test]
    fn zip_slip_and_links_stay_out() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner");
        fs::create_dir(&inner).unwrap();
        let archive = inner.join("evil.zip");
        {
            use zip::write::SimpleFileOptions;
            let mut w = zip::ZipWriter::new(File::create(&archive).unwrap());
            let opts = SimpleFileOptions::default();
            for name in ["../escaped.txt", "/abs.txt", "C:/drive.txt", "ok.txt"] {
                w.start_file(name, opts).unwrap();
                w.write_all(b"x").unwrap();
            }
            w.add_symlink("link", "../../outside", opts).unwrap();
            w.finish().unwrap();
        }
        let done = extract(&archive, ArchiveKind::Zip, Limits::default(), &no_cancel()).unwrap();
        assert_eq!(files_in(&done.folder), ["ok.txt"]);
        assert_eq!(done.skipped, 4, "three unsafe names and a link");
        assert!(!dir.path().join("escaped.txt").exists());
        assert!(!inner.join("escaped.txt").exists());
    }

    #[test]
    fn zip_bombs_stop_and_leave_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("bomb.zip");
        // 8 MiB of zeros compress to a few KiB.
        write_zip(&archive, &[("zeros.bin", &vec![0u8; 8 << 20])]);
        let small = Limits {
            allowance: 1 << 20,
            max_ratio: 100,
            ..Limits::default()
        };
        let err = extract(&archive, ArchiveKind::Zip, small, &no_cancel()).unwrap_err();
        assert!(matches!(err, ExtractError::TooLarge(_)), "{err:?}");
        assert_eq!(err.code(), "too_large");
        assert!(
            !dir.path().join("bomb").exists(),
            "the partial folder is removed"
        );

        let few = Limits {
            max_entries: 1,
            ..Limits::default()
        };
        let two = dir.path().join("two.zip");
        write_zip(&two, &[("a", b"a"), ("b", b"b")]);
        let err = extract(&two, ArchiveKind::Zip, few, &no_cancel()).unwrap_err();
        assert!(matches!(err, ExtractError::TooManyEntries(1)), "{err:?}");
    }

    #[test]
    fn a_header_that_lies_about_the_size_is_caught_while_writing() {
        // Declared sizes are checked first, but only the bytes written count:
        // the budget applies to a stored entry exactly.
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("exact.zip");
        write_zip(&archive, &[("a.bin", &[1u8; 3000])]);
        let tight = Limits {
            allowance: 2000,
            max_ratio: 0,
            ..Limits::default()
        };
        assert!(matches!(
            extract(&archive, ArchiveKind::Zip, tight, &no_cancel()),
            Err(ExtractError::TooLarge(2000))
        ));
    }

    #[test]
    fn damaged_and_canceled_archives_leave_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("broken.zip");
        fs::write(&archive, b"PK\x03\x04 not really a zip").unwrap();
        let err = extract(&archive, ArchiveKind::Zip, Limits::default(), &no_cancel()).unwrap_err();
        assert_eq!(err.code(), "corrupt", "{err:?}");
        assert!(!dir.path().join("broken").exists());

        let ok = dir.path().join("ok.zip");
        write_zip(&ok, &[("a", b"a")]);
        let canceled = AtomicBool::new(true);
        let err = extract(&ok, ArchiveKind::Zip, Limits::default(), &canceled).unwrap_err();
        assert_eq!(err.code(), "canceled");
        assert!(!dir.path().join("ok").exists());
    }

    fn tar_bytes(entries: &[(&str, tar::EntryType, &[u8])]) -> Vec<u8> {
        let mut b = tar::Builder::new(Vec::new());
        for (name, kind, data) in entries {
            let mut h = tar::Header::new_gnu();
            // `set_path` refuses `..`; write the raw name like a hostile tool.
            let raw = &mut h.as_old_mut().name;
            raw[..name.len()].copy_from_slice(name.as_bytes());
            h.set_entry_type(*kind);
            h.set_size(data.len() as u64);
            h.set_mode(if *kind == tar::EntryType::Directory {
                0o755
            } else {
                0o644
            });
            if *kind == tar::EntryType::Symlink {
                h.set_link_name("/etc/passwd").unwrap();
            }
            h.set_cksum();
            b.append(&h, *data).unwrap();
        }
        b.into_inner().unwrap()
    }

    fn hostile_tar() -> Vec<u8> {
        use tar::EntryType as T;
        tar_bytes(&[
            ("dir/", T::Directory, b""),
            ("dir/file.txt", T::Regular, b"data"),
            ("../up.txt", T::Regular, b"x"),
            ("/root.txt", T::Regular, b"x"),
            ("dir/link", T::Symlink, b""),
            ("dir/hard", T::Link, b""),
            ("fifo", T::Fifo, b""),
        ])
    }

    #[test]
    fn tar_gz_and_xz_extract_safely() {
        let dir = tempfile::tempdir().unwrap();
        let tar = hostile_tar();

        let plain = dir.path().join("p.tar");
        fs::write(&plain, &tar).unwrap();

        let gz = dir.path().join("g.tar.gz");
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        enc.write_all(&tar).unwrap();
        fs::write(&gz, enc.finish().unwrap()).unwrap();

        let xz = dir.path().join("x.txz");
        let mut enc =
            lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default()).unwrap();
        enc.write_all(&tar).unwrap();
        fs::write(&xz, enc.finish().unwrap()).unwrap();

        for (path, kind) in [
            (&plain, ArchiveKind::Tar),
            (&gz, ArchiveKind::TarGz),
            (&xz, ArchiveKind::TarXz),
        ] {
            let done = extract(path, kind, Limits::default(), &no_cancel()).unwrap();
            assert_eq!(files_in(&done.folder), ["dir/file.txt"], "{kind:?}");
            assert_eq!(done.skipped, 5, "{kind:?}");
        }
        assert!(!dir.path().join("up.txt").exists());
    }

    #[test]
    fn seven_z_extracts_safely() {
        use sevenz_rust2::{ArchiveEntry, ArchiveWriter};
        let dir = tempfile::tempdir().unwrap();
        let archive = dir.path().join("s.7z");
        {
            let mut w = ArchiveWriter::create(&archive).unwrap();
            for (name, data) in [
                ("docs/a.txt", &b"seven"[..]),
                ("../up.txt", b"x"),
                ("C:\\x.txt", b"x"),
            ] {
                let mut entry = ArchiveEntry::new_file(name);
                entry.size = data.len() as u64;
                w.push_archive_entry(entry, Some(data)).unwrap();
            }
            w.finish().unwrap();
        }
        let done = extract(
            &archive,
            ArchiveKind::SevenZ,
            Limits::default(),
            &no_cancel(),
        )
        .unwrap();
        assert_eq!(files_in(&done.folder), ["docs/a.txt"]);
        assert_eq!(done.skipped, 2);
        assert_eq!(fs::read(done.folder.join("docs/a.txt")).unwrap(), b"seven");
    }
}
