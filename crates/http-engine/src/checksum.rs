use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

use sha2::{Digest, Sha256, Sha512};

use crate::DownloadError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgo {
    Sha256,
    Sha512,
}

impl HashAlgo {
    pub fn name(self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    fn hex_len(self) -> usize {
        match self {
            Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }
}

/// Expected digest, written as `sha256:<hex>` or `sha512:<hex>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksum {
    pub algo: HashAlgo,
    pub hex: String,
}

impl FromStr for Checksum {
    type Err = DownloadError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || DownloadError::InvalidChecksum(s.to_string());
        let (algo, hex) = s.trim().split_once([':', '=']).ok_or_else(invalid)?;
        let algo = match algo.to_ascii_lowercase().as_str() {
            "sha256" | "sha-256" => HashAlgo::Sha256,
            "sha512" | "sha-512" => HashAlgo::Sha512,
            _ => return Err(invalid()),
        };
        let hex = hex.trim().to_ascii_lowercase();
        if hex.len() != algo.hex_len() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        Ok(Self { algo, hex })
    }
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.algo.name(), self.hex)
    }
}

/// Stream the file through the hash; never loads it into memory.
pub fn hash_file(path: &Path, algo: HashAlgo) -> io::Result<String> {
    let mut file = File::open(path)?;
    match algo {
        HashAlgo::Sha256 => digest::<Sha256>(&mut file),
        HashAlgo::Sha512 => digest::<Sha512>(&mut file),
    }
}

fn digest<D: Digest>(reader: &mut impl Read) -> io::Result<String> {
    let mut hasher = D::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_validates() {
        let hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let c: Checksum = format!("SHA256:{}", hex.to_uppercase()).parse().unwrap();
        assert_eq!(c.algo, HashAlgo::Sha256);
        assert_eq!(c.hex, hex);
        assert_eq!(c.to_string(), format!("sha256:{hex}"));
        assert!("sha256:abc".parse::<Checksum>().is_err());
        assert!(
            "md5:d41d8cd98f00b204e9800998ecf8427e"
                .parse::<Checksum>()
                .is_err()
        );
        assert!("nonsense".parse::<Checksum>().is_err());
    }

    #[test]
    fn hashes_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty");
        std::fs::write(&path, b"").unwrap();
        assert_eq!(
            hash_file(&path, HashAlgo::Sha256).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        std::fs::write(&path, b"abc").unwrap();
        assert!(
            hash_file(&path, HashAlgo::Sha512)
                .unwrap()
                .starts_with("ddaf35a193617aba")
        );
    }
}
