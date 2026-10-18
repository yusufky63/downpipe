//! Magnet links (BEP 9): the info hash, a display name, trackers and peers.
//!
//! Parsed here rather than by librqbit because Downpipe needs the info hash
//! before anything is started (to spot a torrent already in the list) and
//! the `x.pe` peers, which librqbit's parser ignores.

use std::net::SocketAddr;

/// A parsed `magnet:` link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Magnet {
    /// BitTorrent v1 info hash, 40 lower-case hex digits.
    pub info_hash: String,
    /// `dn`: what the link calls the torrent; the metadata may differ.
    pub name: Option<String>,
    /// `tr`: tracker URLs, in the link's order, without duplicates.
    pub trackers: Vec<String>,
    /// `x.pe`: peers to try first. Only literal addresses; names would need
    /// a DNS lookup the link's author chose for us.
    pub peers: Vec<SocketAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MagnetError {
    #[error("not a magnet link")]
    NotMagnet,
    #[error("the magnet link has no BitTorrent info hash (xt=urn:btih:...)")]
    NoInfoHash,
    #[error(
        "the magnet link only has a BitTorrent v2 hash (urn:btmh); only v1 and hybrid torrents are supported"
    )]
    V2Only,
    #[error("the magnet link's info hash is not valid")]
    BadInfoHash,
}

/// Whether `input` looks like a magnet link (it may still fail to parse).
pub fn is_magnet(input: &str) -> bool {
    input
        .trim()
        .get(..7)
        .is_some_and(|s| s.eq_ignore_ascii_case("magnet:"))
}

impl Magnet {
    pub fn parse(input: &str) -> Result<Self, MagnetError> {
        let input = input.trim();
        if !is_magnet(input) {
            return Err(MagnetError::NotMagnet);
        }
        // `magnet:?a=b`: everything after the first `?` is the query.
        let query = input.split_once('?').map_or("", |(_, q)| q);
        let mut info_hash = None;
        let mut v2 = false;
        let mut name = None;
        let mut trackers: Vec<String> = Vec::new();
        let mut peers = Vec::new();
        for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
            let key = key.to_ascii_lowercase();
            // `xt.1`, `tr.2`: numbered forms some clients write.
            let key = if key == "x.pe" {
                key
            } else {
                key.split('.').next().unwrap_or_default().to_string()
            };
            match key.as_str() {
                "xt" => {
                    let lower = value.to_ascii_lowercase();
                    if let Some(hash) = lower.strip_prefix("urn:btih:") {
                        if info_hash.is_none() {
                            info_hash = Some(decode_hash(hash).ok_or(MagnetError::BadInfoHash)?);
                        }
                    } else if lower.starts_with("urn:btmh:") {
                        v2 = true;
                    }
                }
                "dn" => {
                    let value = value.trim();
                    if !value.is_empty() && name.is_none() {
                        name = Some(value.to_string());
                    }
                }
                "tr" => {
                    let value = value.trim();
                    if !value.is_empty() && !trackers.iter().any(|t| t == value) {
                        trackers.push(value.to_string());
                    }
                }
                "x.pe" => {
                    if let Ok(addr) = value.trim().parse::<SocketAddr>()
                        && !peers.contains(&addr)
                    {
                        peers.push(addr);
                    }
                }
                _ => {}
            }
        }
        let info_hash = match info_hash {
            Some(hash) => hash,
            None if v2 => return Err(MagnetError::V2Only),
            None => return Err(MagnetError::NoInfoHash),
        };
        Ok(Self {
            info_hash,
            name,
            trackers,
            peers,
        })
    }

    /// The link as Downpipe stores it: hash, name, trackers and peers, in a
    /// fixed order, so the same torrent always gets the same link.
    pub fn to_link(&self) -> String {
        let mut out = format!("magnet:?xt=urn:btih:{}", self.info_hash);
        let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
        if let Some(name) = &self.name {
            out.push_str("&dn=");
            out.push_str(&enc(name));
        }
        for tracker in &self.trackers {
            out.push_str("&tr=");
            out.push_str(&enc(tracker));
        }
        for peer in &self.peers {
            out.push_str("&x.pe=");
            out.push_str(&enc(&peer.to_string()));
        }
        out
    }
}

/// 40 hex digits or 32 base32 letters (both appear in the wild) to hex.
fn decode_hash(hash: &str) -> Option<String> {
    let hash = hash.trim();
    if hash.len() == 40 && hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Some(hash.to_ascii_lowercase());
    }
    if hash.len() == 32 {
        let bytes = base32(hash)?;
        return Some(bytes.iter().map(|b| format!("{b:02x}")).collect());
    }
    None
}

/// RFC 4648 base32 without padding, as BEP 9 allows for 20-byte hashes.
fn base32(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(input.len() * 5 / 8);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for c in input.bytes() {
        let value = match c.to_ascii_uppercase() {
            c @ b'A'..=b'Z' => c - b'A',
            c @ b'2'..=b'7' => c - b'2' + 26,
            _ => return None,
        };
        buffer = (buffer << 5) | u32::from(value);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const HASH: &str = "c9e15763f722f23e98a29decdfae341b98d53056";

    #[test]
    fn parses_hash_name_trackers_and_peers() {
        let m = Magnet::parse(&format!(
            "magnet:?xt=urn:btih:{}&dn=Big+Buck%20Bunny&tr=udp%3A%2F%2Ftracker.example%3A1337&tr=udp%3A%2F%2Ftracker.example%3A1337&tr=https://t.example/announce&x.pe=127.0.0.1:6881&x.pe=[::1]:51413&x.pe=peer.example:1",
            HASH.to_ascii_uppercase()
        ))
        .unwrap();
        assert_eq!(m.info_hash, HASH);
        assert_eq!(m.name.as_deref(), Some("Big Buck Bunny"));
        assert_eq!(
            m.trackers,
            ["udp://tracker.example:1337", "https://t.example/announce"]
        );
        assert_eq!(
            m.peers,
            [
                "127.0.0.1:6881".parse::<SocketAddr>().unwrap(),
                "[::1]:51413".parse().unwrap()
            ]
        );
    }

    #[test]
    fn decodes_base32_hashes() {
        // The same 20 bytes as HASH, in base32.
        let m = Magnet::parse("magnet:?xt=urn:btih:ZHQVOY7XELZD5GFCTXWN7LRUDOMNKMCW").unwrap();
        assert_eq!(m.info_hash, HASH);
    }

    #[test]
    fn refuses_links_without_a_v1_hash() {
        assert_eq!(
            Magnet::parse("https://example.org"),
            Err(MagnetError::NotMagnet)
        );
        assert_eq!(Magnet::parse("magnet:?dn=x"), Err(MagnetError::NoInfoHash));
        assert_eq!(
            Magnet::parse("magnet:?xt=urn:btmh:1220abcd"),
            Err(MagnetError::V2Only)
        );
        assert_eq!(
            Magnet::parse("magnet:?xt=urn:btih:nothex"),
            Err(MagnetError::BadInfoHash)
        );
    }

    #[test]
    fn link_round_trips() {
        let m = Magnet::parse(&format!(
            "MAGNET:?dn=a%26b&xt=urn:btih:{HASH}&tr=udp://t.example:1&x.pe=10.0.0.1:2"
        ))
        .unwrap();
        let link = m.to_link();
        assert!(link.starts_with(&format!("magnet:?xt=urn:btih:{HASH}&dn=a%26b&tr=")));
        assert_eq!(Magnet::parse(&link).unwrap(), m);
        assert!(is_magnet(" magnet:?xt"));
        assert!(!is_magnet("magnet"));
    }
}
