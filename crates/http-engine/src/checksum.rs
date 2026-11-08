use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use md5::Md5;
use reqwest::header::HeaderMap;
use sha2::{Digest, Sha256, Sha512};

use crate::DownloadError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HashAlgo {
    /// Only to catch a corrupt transfer, which it still does well: what
    /// some sites and storage services (Google Cloud Storage) publish.
    Md5,
    Sha256,
    Sha512,
}

impl HashAlgo {
    pub fn name(self) -> &'static str {
        match self {
            Self::Md5 => "md5",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    fn hex_len(self) -> usize {
        match self {
            Self::Md5 => 32,
            Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "md5" => Some(Self::Md5),
            "sha256" | "sha-256" => Some(Self::Sha256),
            "sha512" | "sha-512" => Some(Self::Sha512),
            _ => None,
        }
    }
}

/// Expected digest, written as `sha256:<hex>`, `sha512:<hex>` or `md5:<hex>`.
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
        let algo = HashAlgo::from_name(algo).ok_or_else(invalid)?;
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
        HashAlgo::Md5 => digest::<Md5>(&mut file),
        HashAlgo::Sha256 => digest::<Sha256>(&mut file),
        HashAlgo::Sha512 => digest::<Sha512>(&mut file),
    }
}

/// The digest of the whole file a server announces with its answer, the
/// strongest one when it gives several: `Repr-Digest` (RFC 9530) and
/// `Digest` (RFC 3230) describe the whole representation even in a `206`;
/// `x-goog-hash` (Google Cloud Storage) the whole object; `Content-MD5` only
/// the body it comes with, so it counts only when `whole_body` says that
/// body is the whole file.
pub(crate) fn announced(headers: &HeaderMap, whole_body: bool) -> Option<Checksum> {
    let mut found: Vec<(HashAlgo, Vec<u8>)> = Vec::new();
    for value in headers.get_all("repr-digest") {
        let Ok(value) = value.to_str() else { continue };
        for item in value.split(',') {
            // sha-256=:BASE64: (a structured field byte sequence)
            if let Some((name, bytes)) = item.split_once('=')
                && let Some(algo) = HashAlgo::from_name(name)
                && let Some(b64) = bytes
                    .trim()
                    .strip_prefix(':')
                    .and_then(|b| b.strip_suffix(':'))
                && let Ok(raw) = BASE64.decode(b64)
            {
                found.push((algo, raw));
            }
        }
    }
    for (header, only) in [("digest", None), ("x-goog-hash", Some(HashAlgo::Md5))] {
        for value in headers.get_all(header) {
            let Ok(value) = value.to_str() else { continue };
            for item in value.split(',') {
                if let Some((name, b64)) = item.split_once('=')
                    && let Some(algo) = HashAlgo::from_name(name)
                    && only.is_none_or(|o| o == algo)
                    && let Ok(raw) = BASE64.decode(b64.trim())
                {
                    found.push((algo, raw));
                }
            }
        }
    }
    if whole_body
        && let Some(value) = headers.get("content-md5").and_then(|v| v.to_str().ok())
        && let Ok(raw) = BASE64.decode(value.trim())
    {
        found.push((HashAlgo::Md5, raw));
    }
    found
        .into_iter()
        .filter(|(algo, raw)| raw.len() * 2 == algo.hex_len())
        .max_by_key(|(algo, _)| *algo)
        .map(|(algo, raw)| Checksum {
            algo,
            hex: hex::encode(raw),
        })
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
        let md5: Checksum = "md5:D41D8CD98F00B204E9800998ECF8427E".parse().unwrap();
        assert_eq!(md5.algo, HashAlgo::Md5);
        assert_eq!(md5.hex, "d41d8cd98f00b204e9800998ecf8427e");
        assert!("md5:d41d8cd98f00b204".parse::<Checksum>().is_err());
        assert!(
            "crc32:d41d8cd98f00b204e9800998ecf8427e"
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
        assert_eq!(
            hash_file(&path, HashAlgo::Md5).unwrap(),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
        std::fs::write(&path, b"abc").unwrap();
        assert!(
            hash_file(&path, HashAlgo::Sha512)
                .unwrap()
                .starts_with("ddaf35a193617aba")
        );
    }
}

#[cfg(test)]
mod announced_tests {
    use super::*;
    use reqwest::header::HeaderValue;

    // sha256 and md5 of "abc".
    const SHA256_B64: &str = "ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=";
    const SHA256_HEX: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const MD5_B64: &str = "kAFQmDzST7DWlj99KOF/cg==";
    const MD5_HEX: &str = "900150983cd24fb0d6963f7d28e17f72";

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (name, value) in pairs {
            h.append(*name, HeaderValue::from_str(value).unwrap());
        }
        h
    }

    #[test]
    fn reads_the_strongest_digest_a_server_announces() {
        let goog = format!("crc32c=n03x6A==, md5={MD5_B64}");
        let repr = format!("sha-256=:{SHA256_B64}:");
        let h = headers(&[("x-goog-hash", &goog), ("repr-digest", &repr)]);
        let c = announced(&h, false).unwrap();
        assert_eq!((c.algo, c.hex.as_str()), (HashAlgo::Sha256, SHA256_HEX));

        let digest = format!("SHA-256={SHA256_B64}");
        let h = headers(&[("digest", &digest)]);
        assert_eq!(announced(&h, false).unwrap().hex, SHA256_HEX);

        let md5 = format!("md5={MD5_B64}");
        let h = headers(&[("x-goog-hash", "crc32c=n03x6A=="), ("x-goog-hash", &md5)]);
        let c = announced(&h, false).unwrap();
        assert_eq!((c.algo, c.hex.as_str()), (HashAlgo::Md5, MD5_HEX));
    }

    #[test]
    fn content_md5_counts_only_for_the_whole_file() {
        let h = headers(&[("content-md5", MD5_B64)]);
        assert_eq!(announced(&h, false), None, "the hash of a part");
        assert_eq!(announced(&h, true).unwrap().hex, MD5_HEX);
    }

    #[test]
    fn ignores_what_it_cannot_use() {
        let goog = format!("sha-256={SHA256_B64}");
        let h = headers(&[
            ("repr-digest", "sha-256=:not base64:, unixsum=:MTIz:"),
            ("digest", "sha-256=YWJj"),
            ("x-goog-hash", &goog),
        ]);
        assert_eq!(announced(&h, true), None);
    }
}
