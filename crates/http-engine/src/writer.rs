//! Positional writes into the `.part` file.
//!
//! Several connections write to different offsets of one file at the same
//! time, so writes are positional (`pwrite` / `WriteFile` with an offset)
//! rather than seek + write on a shared cursor.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[derive(Debug)]
pub struct PartFile {
    file: File,
}

impl PartFile {
    /// Create (or truncate) a part file sized for `total` bytes.
    pub fn create(path: &Path, total: Option<u64>) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let part = Self { file };
        if let Some(total) = total {
            part.prepare(total)?;
        }
        Ok(part)
    }

    /// Open an existing part file for resuming. Its length must match.
    pub fn open_existing(path: &Path, total: u64) -> io::Result<Self> {
        let file = OpenOptions::new().read(true).write(true).open(path)?;
        let len = file.metadata()?.len();
        if len != total {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("part file is {len} bytes, expected {total}"),
            ));
        }
        Ok(Self { file })
    }

    fn prepare(&self, total: u64) -> io::Result<()> {
        // On NTFS, writing far past the valid-data length makes the kernel
        // zero-fill everything before it synchronously; with segments at
        // 7/8 of a 10 GB file that stalls the writer for seconds. A sparse
        // file skips the zero-fill. Best effort: FAT/exFAT do not support it.
        #[cfg(windows)]
        set_sparse(&self.file);
        self.file.set_len(total)
    }

    pub fn write_all_at(&self, mut offset: u64, mut buf: &[u8]) -> io::Result<()> {
        while !buf.is_empty() {
            let n = write_at(&self.file, buf, offset)?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "failed to write to part file",
                ));
            }
            buf = &buf[n..];
            offset += n as u64;
        }
        Ok(())
    }

    /// Flush data to stable storage; called before the meta claims it.
    pub fn sync(&self) -> io::Result<()> {
        self.file.sync_data()
    }

    pub fn truncate(&self, len: u64) -> io::Result<()> {
        self.file.set_len(len)
    }
}

#[cfg(unix)]
fn write_at(file: &File, buf: &[u8], offset: u64) -> io::Result<usize> {
    use std::os::unix::fs::FileExt;
    file.write_at(buf, offset)
}

#[cfg(windows)]
fn write_at(file: &File, buf: &[u8], offset: u64) -> io::Result<usize> {
    use std::os::windows::fs::FileExt;
    file.seek_write(buf, offset)
}

#[cfg(windows)]
fn set_sparse(file: &File) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::FSCTL_SET_SPARSE;

    let mut returned = 0u32;
    // SAFETY: the handle is valid for the lifetime of `file`; FSCTL_SET_SPARSE
    // with no input buffer means "set sparse = true" and writes no output.
    unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            FSCTL_SET_SPARSE,
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            0,
            &mut returned,
            std::ptr::null_mut(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn positional_writes_land_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.part");
        let part = PartFile::create(&path, Some(10)).unwrap();
        part.write_all_at(6, b"6789").unwrap();
        part.write_all_at(0, b"012345").unwrap();
        part.sync().unwrap();
        drop(part);
        let mut s = String::new();
        File::open(&path).unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "0123456789");

        assert!(PartFile::open_existing(&path, 10).is_ok());
        assert!(PartFile::open_existing(&path, 11).is_err());
    }
}
