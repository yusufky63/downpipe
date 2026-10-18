//! Access tokens: made from the system's random source, shown to the user
//! once, and stored only as a SHA-256 hash.

use sha2::{Digest, Sha256};

/// 160 random bits: far beyond guessing, short enough to read off a screen.
const TOKEN_BYTES: usize = 20;
/// RFC 4648 base32 in lower case: no symbols a phone keyboard hides, and
/// 20 bytes fill exactly 32 characters.
const ALPHABET: &[u8; 32] = b"abcdefghijklmnopqrstuvwxyz234567";
/// Anything longer cannot be a token; it is refused before hashing.
const MAX_INPUT: usize = 128;

/// A new random token, 32 characters of base32.
pub fn generate() -> String {
    let mut bytes = [0u8; TOKEN_BYTES];
    getrandom::fill(&mut bytes).expect("the system's random source is unavailable");
    base32(&bytes)
}

/// What is stored in the settings: SHA-256 of the normalized token, as hex.
pub fn hash(token: &str) -> String {
    hex::encode(digest(&normalize(token)))
}

/// The hash in the settings as bytes, if it is one.
pub fn parse_hash(hex_hash: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(hex_hash).ok()?;
    bytes.try_into().ok()
}

/// `abcd-efgh-...` for reading aloud or copying by hand; dashes and spaces
/// are ignored when a token is checked.
pub fn grouped(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    chars
        .chunks(4)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Whether `candidate` is the token whose hash is `expected`. The hashes are
/// compared in constant time, so response times say nothing about how close
/// a guess was.
pub(crate) fn matches(candidate: &str, expected: &[u8; 32]) -> bool {
    if candidate.len() > MAX_INPUT {
        return false;
    }
    let actual = digest(&normalize(candidate));
    actual
        .iter()
        .zip(expected)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn normalize(token: &str) -> String {
    token
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

fn digest(text: &str) -> [u8; 32] {
    Sha256::digest(text.as_bytes()).into()
}

fn base32(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[((buffer >> bits) & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_long_random_and_base32() {
        let a = generate();
        let b = generate();
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| ALPHABET.contains(&c)));
    }

    #[test]
    fn base32_matches_rfc_4648() {
        // RFC 4648 section 10, in lower case and without padding.
        assert_eq!(base32(b"f"), "my");
        assert_eq!(base32(b"foo"), "mzxw6");
        assert_eq!(base32(b"foobar"), "mzxw6ytboi");
    }

    #[test]
    fn checks_against_the_stored_hash() {
        let token = generate();
        let stored = parse_hash(&hash(&token)).unwrap();
        assert!(matches(&token, &stored));
        assert!(
            matches(&grouped(&token).to_uppercase(), &stored),
            "dashes and case do not matter"
        );
        assert!(!matches(&token[1..], &stored));
        assert!(!matches("", &stored));
        assert!(!matches(&"a".repeat(MAX_INPUT + 1), &stored));
    }

    #[test]
    fn hashes_parse_only_when_complete() {
        assert!(parse_hash(&"ab".repeat(32)).is_some());
        assert!(parse_hash(&"ab".repeat(31)).is_none());
        assert!(parse_hash("zz").is_none());
    }

    #[test]
    fn groups_by_four() {
        assert_eq!(grouped("abcdefghij"), "abcd-efgh-ij");
    }
}
