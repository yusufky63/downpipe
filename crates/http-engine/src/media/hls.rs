//! HLS playlists (RFC 8216): master playlists list the qualities, media
//! playlists list the segments of one of them.
//!
//! Parsing is strict wherever a mistake would corrupt the output (keys, IVs,
//! byte ranges, sequence numbers) and lenient about informational attributes
//! such as bandwidth or resolution, which players ignore when malformed too.

use url::Url;

use super::{ByteSpan, KeyRef, Segment};
use crate::DownloadError;

/// Far more segments than a real on-demand stream has (ten hours in
/// 2-second segments is 18 000); past this the playlist is broken or hostile.
const MAX_SEGMENTS: usize = 100_000;

/// Durations are only added up for estimates. No real segment lasts a week,
/// and the cap keeps the sum of [`MAX_SEGMENTS`] of them far from `u64::MAX`.
const MAX_SEGMENT_SECONDS: f64 = 7.0 * 24.0 * 3600.0;

// A playlist is parsed once per fetch and matched on right away; boxing the
// media variant would only add an allocation and a deref to every caller.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Playlist {
    Master(MasterPlaylist),
    Media(MediaPlaylist),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MasterPlaylist {
    /// `#EXT-X-STREAM-INF` entries in document order (I-frame streams left out).
    pub variants: Vec<Variant>,
    /// `#EXT-X-MEDIA:TYPE=AUDIO` entries in document order.
    pub audio: Vec<Rendition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    pub uri: Url,
    pub bandwidth: Option<u64>,
    pub average_bandwidth: Option<u64>,
    /// `RESOLUTION=1280x720` as `(1280, 720)`.
    pub resolution: Option<(u32, u32)>,
    pub codecs: Option<String>,
    /// `FRAME-RATE` as written, e.g. `"29.970"`.
    pub frame_rate: Option<String>,
    /// `AUDIO="group"`: the [`Rendition::group_id`] of its separate audio.
    pub audio_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendition {
    pub group_id: String,
    pub name: Option<String>,
    pub language: Option<String>,
    /// None: the audio of this group is inside the variants themselves.
    pub uri: Option<Url>,
    pub default: bool,
    pub autoselect: bool,
    pub channels: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaPlaylist {
    pub segments: Vec<Segment>,
    /// `#EXT-X-MAP`: the initialization section (fragmented MP4 streams).
    pub init: Option<Segment>,
    pub target_duration: Option<u64>,
    pub media_sequence: u64,
}

/// Parse a playlist fetched from `base` (relative URIs resolve against it).
pub fn parse(text: &str, base: &Url) -> Result<Playlist, DownloadError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    let Some((&"#EXTM3U", lines)) = lines.split_first() else {
        return Err(invalid("this is not an HLS playlist"));
    };
    // A protected stream is reported as protected whatever else is wrong
    // with it, so keys are checked before anything else.
    refuse_drm(lines)?;
    let has = |tag: &str| tags(lines).any(|(name, _)| name == tag);
    let master = has("#EXT-X-STREAM-INF") || has("#EXT-X-I-FRAME-STREAM-INF");
    match (master, has("#EXTINF")) {
        (true, true) => Err(invalid("the playlist lists both variants and segments")),
        (true, false) => parse_master(lines, base).map(Playlist::Master),
        (false, _) => parse_media(lines, base).map(Playlist::Media),
    }
}

fn parse_master(lines: &[&str], base: &Url) -> Result<MasterPlaylist, DownloadError> {
    let mut variants = Vec::new();
    let mut audio = Vec::new();
    // Attributes of the last `#EXT-X-STREAM-INF`, waiting for its URI line.
    let mut stream_inf = None;
    for &line in lines {
        match split_tag(line) {
            None => {
                // A URI after anything but `#EXT-X-STREAM-INF` describes
                // nothing a player would use.
                if let Some(attrs) = stream_inf.take() {
                    variants.push(variant(&attrs, resolve(base, line)?));
                }
            }
            Some(("#EXT-X-STREAM-INF", value)) => {
                if stream_inf.replace(Attributes::parse(value)).is_some() {
                    return Err(invalid("a variant has no URI"));
                }
            }
            Some(("#EXT-X-MEDIA", value)) => {
                if let Some(rendition) = audio_rendition(&Attributes::parse(value), base)? {
                    audio.push(rendition);
                }
            }
            Some(_) => {}
        }
    }
    if stream_inf.is_some() {
        return Err(invalid("a variant has no URI"));
    }
    if variants.is_empty() {
        return Err(invalid("the master playlist lists no variants"));
    }
    Ok(MasterPlaylist { variants, audio })
}

fn variant(attrs: &Attributes<'_>, uri: Url) -> Variant {
    Variant {
        uri,
        bandwidth: attrs.number("BANDWIDTH"),
        average_bandwidth: attrs.number("AVERAGE-BANDWIDTH"),
        resolution: attrs.get("RESOLUTION").and_then(resolution),
        codecs: attrs.text("CODECS"),
        frame_rate: attrs.text("FRAME-RATE"),
        audio_group: attrs.text("AUDIO"),
    }
}

/// `None` for renditions other than audio (subtitles, captions, video
/// angles): nothing downloads them.
fn audio_rendition(attrs: &Attributes<'_>, base: &Url) -> Result<Option<Rendition>, DownloadError> {
    if attrs.get("TYPE") != Some("AUDIO") {
        return Ok(None);
    }
    let group_id = attrs
        .text("GROUP-ID")
        .ok_or_else(|| invalid("an audio rendition has no GROUP-ID"))?;
    Ok(Some(Rendition {
        group_id,
        name: attrs.text("NAME"),
        language: attrs.text("LANGUAGE"),
        uri: attrs.get("URI").map(|uri| resolve(base, uri)).transpose()?,
        default: attrs.get("DEFAULT") == Some("YES"),
        autoselect: attrs.get("AUTOSELECT") == Some("YES"),
        channels: attrs.text("CHANNELS"),
    }))
}

/// `RESOLUTION=1280x720`.
fn resolution(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once(['x', 'X'])?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn parse_media(lines: &[&str], base: &Url) -> Result<MediaPlaylist, DownloadError> {
    // Checked before the segments: a live window often starts in the middle
    // of a byte-range run, which would otherwise read as a broken playlist.
    if !is_finished(lines) {
        return Err(DownloadError::LiveStream);
    }
    let mut parser = MediaParser::new(base);
    for &line in lines {
        parser.line(line)?;
    }
    parser.finish()
}

/// Only a playlist that promises not to grow can be downloaded in full; an
/// `EVENT` playlist without `#EXT-X-ENDLIST` is still being appended to.
fn is_finished(lines: &[&str]) -> bool {
    tags(lines).any(|tag| matches!(tag, ("#EXT-X-ENDLIST", _) | ("#EXT-X-PLAYLIST-TYPE", "VOD")))
}

/// State of a media playlist read top to bottom.
#[derive(Debug)]
struct MediaParser<'a> {
    base: &'a Url,
    media_sequence: u64,
    target_duration: Option<u64>,
    segments: Vec<Segment>,
    init: Option<InitSection>,
    key: Option<AesKey>,
    next: NextSegment,
    /// `#EXTINF` segments seen so far, gaps included: the index of the next.
    count: u64,
    /// URL and end of the previous segment's byte range, which a
    /// `#EXT-X-BYTERANGE` without an offset continues.
    range_end: Option<(Url, u64)>,
}

/// Tags seen since the last URI line: they describe the segment whose URI
/// comes next.
#[derive(Debug, Default)]
struct NextSegment {
    duration_ms: Option<u64>,
    range: Option<RangeSpec>,
    gap: bool,
}

/// `#EXT-X-MAP` as written. Its IV is settled in `finish` because
/// `#EXT-X-MEDIA-SEQUENCE` may still follow it.
#[derive(Debug)]
struct InitSection {
    url: Url,
    range: Option<ByteSpan>,
    key: Option<AesKey>,
    /// Index of the first segment the map applies to.
    next_index: u64,
}

impl<'a> MediaParser<'a> {
    fn new(base: &'a Url) -> Self {
        Self {
            base,
            media_sequence: 0,
            target_duration: None,
            segments: Vec::new(),
            init: None,
            key: None,
            next: NextSegment::default(),
            count: 0,
            range_end: None,
        }
    }

    fn line(&mut self, line: &str) -> Result<(), DownloadError> {
        let Some((name, value)) = split_tag(line) else {
            return self.push_segment(line);
        };
        match name {
            "#EXTINF" => self.next.duration_ms = Some(duration_ms(value)?),
            "#EXT-X-BYTERANGE" => self.next.range = Some(RangeSpec::parse(value)?),
            "#EXT-X-GAP" => self.next.gap = true,
            "#EXT-X-KEY" => self.key = aes_key(&Attributes::parse(value), self.base)?,
            "#EXT-X-MAP" => self.set_init(&Attributes::parse(value))?,
            "#EXT-X-MEDIA-SEQUENCE" => self.set_media_sequence(value)?,
            "#EXT-X-TARGETDURATION" => self.target_duration = value.parse().ok(),
            // Discontinuities, dates, low-latency parts and the like change
            // nothing about which bytes make up the stream.
            _ => {}
        }
        Ok(())
    }

    fn set_media_sequence(&mut self, value: &str) -> Result<(), DownloadError> {
        // The number feeds the implicit IVs, so it cannot change under
        // segments that already used it.
        if self.count > 0 {
            return Err(invalid("EXT-X-MEDIA-SEQUENCE comes after a segment"));
        }
        self.media_sequence = value
            .parse()
            .map_err(|_| invalid("EXT-X-MEDIA-SEQUENCE is not a valid number"))?;
        Ok(())
    }

    fn set_init(&mut self, attrs: &Attributes<'_>) -> Result<(), DownloadError> {
        let uri = attrs
            .get("URI")
            .ok_or_else(|| invalid("EXT-X-MAP has no URI"))?;
        let url = resolve(self.base, uri)?;
        let range = match attrs.get("BYTERANGE") {
            Some(value) => {
                let spec = RangeSpec::parse(value)?;
                Some(byte_span(spec.offset.unwrap_or(0), spec.length)?.0)
            }
            None => None,
        };
        match &self.init {
            // Packagers repeat an unchanged map after discontinuities.
            Some(init) if init.url == url && init.range == range => Ok(()),
            // One output file can only start with one initialization section.
            Some(_) => Err(DownloadError::UnsupportedMedia(
                "the playlist changes its initialization section".to_owned(),
            )),
            None => {
                self.init = Some(InitSection {
                    url,
                    range,
                    key: self.key.clone(),
                    next_index: self.count,
                });
                Ok(())
            }
        }
    }

    fn push_segment(&mut self, uri: &str) -> Result<(), DownloadError> {
        let next = std::mem::take(&mut self.next);
        let duration_ms = next
            .duration_ms
            .ok_or_else(|| invalid("a segment has no EXTINF"))?;
        let url = resolve(self.base, uri)?;
        // A gap still takes its sequence number and its bytes of a shared
        // resource, so it advances both before it is dropped.
        let range = self.continue_range(&url, next.range)?;
        let number = sequence(self.media_sequence, self.count);
        self.count += 1;
        if next.gap {
            return Ok(());
        }
        if self.segments.len() >= MAX_SEGMENTS {
            return Err(DownloadError::UnsupportedMedia(
                "the playlist has too many segments".to_owned(),
            ));
        }
        self.segments.push(Segment {
            url,
            range,
            key: self.key.as_ref().map(|key| key.for_sequence(number)),
            duration_ms,
        });
        Ok(())
    }

    /// Without an offset, a range starts where the previous segment's range
    /// ended, and that segment must be a range of the same resource.
    fn continue_range(
        &mut self,
        url: &Url,
        range: Option<RangeSpec>,
    ) -> Result<Option<ByteSpan>, DownloadError> {
        let previous = self.range_end.take();
        let Some(RangeSpec { length, offset }) = range else {
            return Ok(None);
        };
        let offset = match (offset, previous) {
            (Some(offset), _) => offset,
            (None, Some((previous_url, end))) if previous_url == *url => end,
            (None, _) => {
                return Err(invalid(
                    "a byte range without an offset does not follow a range of the same URI",
                ));
            }
        };
        let (span, end) = byte_span(offset, length)?;
        self.range_end = Some((url.clone(), end));
        Ok(Some(span))
    }

    fn finish(self) -> Result<MediaPlaylist, DownloadError> {
        if self.next.duration_ms.is_some() {
            return Err(invalid("the last segment has no URI"));
        }
        if self.segments.is_empty() {
            return Err(invalid("the playlist has no segments"));
        }
        let media_sequence = self.media_sequence;
        let init = self.init.map(|init| Segment {
            url: init.url,
            range: init.range,
            key: init
                .key
                .map(|key| key.for_sequence(sequence(media_sequence, init.next_index))),
            duration_ms: 0,
        });
        Ok(MediaPlaylist {
            segments: self.segments,
            init,
            target_duration: self.target_duration,
            media_sequence,
        })
    }
}

/// Media sequence number of the segment at `index`. Kept in 128 bits, the
/// width of the IV it becomes, so no playlist value can overflow it.
fn sequence(media_sequence: u64, index: u64) -> u128 {
    u128::from(media_sequence) + u128::from(index)
}

/// `#EXTINF:<seconds>[,<title>]`.
fn duration_ms(value: &str) -> Result<u64, DownloadError> {
    let seconds = value.split_once(',').map_or(value, |(seconds, _)| seconds);
    match seconds.trim().parse::<f64>() {
        // `as` saturates, and the range check keeps it from having to.
        Ok(s) if (0.0..=MAX_SEGMENT_SECONDS).contains(&s) => Ok((s * 1000.0).round() as u64),
        _ => Err(invalid("a segment has no valid EXTINF duration")),
    }
}

/// `n[@o]` of `#EXT-X-BYTERANGE` and of the `BYTERANGE` attribute of
/// `#EXT-X-MAP`.
#[derive(Debug, Clone, Copy)]
struct RangeSpec {
    length: u64,
    offset: Option<u64>,
}

impl RangeSpec {
    fn parse(value: &str) -> Result<Self, DownloadError> {
        let number = |s: &str| {
            s.trim()
                .parse::<u64>()
                .map_err(|_| invalid("a byte range is not valid"))
        };
        Ok(match value.split_once('@') {
            Some((length, offset)) => Self {
                length: number(length)?,
                offset: Some(number(offset)?),
            },
            None => Self {
                length: number(value)?,
                offset: None,
            },
        })
    }
}

/// A non-empty span together with its end, checked to fit in a `u64` so
/// neither the next offset nor `ByteSpan::last` can overflow.
fn byte_span(offset: u64, length: u64) -> Result<(ByteSpan, u64), DownloadError> {
    if length == 0 {
        return Err(invalid("a byte range is empty"));
    }
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid("a byte range is out of bounds"))?;
    Ok((ByteSpan { offset, length }, end))
}

/// What a key tag asks for.
#[derive(Debug, Clone, Copy)]
enum KeyMethod {
    Clear,
    Aes128,
}

/// Plain AES-128 with a key served as-is (`identity`) is the only
/// encryption that can be undone without a DRM system. Every other method
/// or key format, including ones this parser has never heard of, is DRM
/// and refused: skipping it would produce files that do not play.
fn key_method(attrs: &Attributes<'_>) -> Result<KeyMethod, DownloadError> {
    let identity = attrs
        .get("KEYFORMAT")
        .is_none_or(|format| format == "identity");
    match attrs.get("METHOD") {
        Some("NONE") => Ok(KeyMethod::Clear),
        Some("AES-128") if identity => Ok(KeyMethod::Aes128),
        Some(_) => Err(DownloadError::Protected),
        None => Err(invalid("a key has no METHOD")),
    }
}

/// `#EXT-X-KEY` and `#EXT-X-SESSION-KEY` are checked in both kinds of
/// playlist: a misplaced DRM key still means DRM.
fn refuse_drm(lines: &[&str]) -> Result<(), DownloadError> {
    for (name, value) in tags(lines) {
        if name == "#EXT-X-KEY" || name == "#EXT-X-SESSION-KEY" {
            key_method(&Attributes::parse(value))?;
        }
    }
    Ok(())
}

/// An `#EXT-X-KEY:METHOD=AES-128` in effect.
#[derive(Debug, Clone)]
struct AesKey {
    uri: Url,
    iv: Option<[u8; 16]>,
}

impl AesKey {
    /// Without an `IV` attribute, a segment's IV is its media sequence
    /// number as a big-endian 128-bit integer (RFC 8216 §5.2).
    fn for_sequence(&self, sequence: u128) -> KeyRef {
        KeyRef {
            uri: self.uri.clone(),
            iv: self.iv.unwrap_or(sequence.to_be_bytes()),
        }
    }
}

/// The key an `#EXT-X-KEY` puts in effect; `None` after `METHOD=NONE`.
fn aes_key(attrs: &Attributes<'_>, base: &Url) -> Result<Option<AesKey>, DownloadError> {
    match key_method(attrs)? {
        KeyMethod::Clear => Ok(None),
        KeyMethod::Aes128 => {
            let uri = attrs
                .get("URI")
                .ok_or_else(|| invalid("an AES-128 key has no URI"))?;
            Ok(Some(AesKey {
                uri: resolve(base, uri)?,
                iv: attrs.get("IV").map(parse_iv).transpose()?,
            }))
        }
    }
}

/// `IV=0x…`: up to 32 hex digits read as a 128-bit big-endian number, which
/// left-pads short values with zeros.
fn parse_iv(value: &str) -> Result<[u8; 16], DownloadError> {
    let bad = || invalid("a key IV is not a 128-bit hexadecimal number");
    let digits = value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
        .ok_or_else(bad)?;
    // `from_str_radix` would also take a leading `+`.
    if digits.is_empty() || digits.len() > 32 || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    u128::from_str_radix(digits, 16)
        .map(u128::to_be_bytes)
        .map_err(|_| bad())
}

/// `uri` against `base`. Only http(s) is downloaded: a `file:` or `data:`
/// URI in a fetched playlist must never reach the network layer.
fn resolve(base: &Url, uri: &str) -> Result<Url, DownloadError> {
    // An empty URI would resolve to the playlist itself.
    if uri.is_empty() {
        return Err(invalid("a URI in the playlist is empty"));
    }
    // The URI itself stays out of the message: it may carry access tokens.
    let url = base
        .join(uri)
        .map_err(|_| invalid("a URI in the playlist is not valid"))?;
    match url.scheme() {
        "http" | "https" => Ok(url),
        other => Err(DownloadError::InvalidManifest(format!(
            "a URI in the playlist uses `{other}:`; only http and https are supported"
        ))),
    }
}

/// Tag lines split into name and value.
fn tags<'a>(lines: &'a [&'a str]) -> impl Iterator<Item = (&'a str, &'a str)> + 'a {
    lines.iter().copied().filter_map(split_tag)
}

/// `("#EXT-X-FOO", "value")` for a `#EXT-X-FOO:value` line, `None` for a
/// URI line. Comments come out as tags nobody asks for.
fn split_tag(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with('#') {
        return None;
    }
    Some(match line.split_once(':') {
        Some((name, value)) => (name, value.trim()),
        None => (line, ""),
    })
}

/// The `NAME=value,...` list of a tag. Quoted values keep their commas and
/// `=` and lose their quotes. Malformed entries are skipped rather than
/// fatal: a stray comma should not cost a download, and every attribute
/// that matters is validated where it is used.
#[derive(Debug)]
struct Attributes<'a>(Vec<(&'a str, &'a str)>);

impl<'a> Attributes<'a> {
    fn parse(mut list: &'a str) -> Self {
        let mut pairs = Vec::new();
        while !list.is_empty() {
            let (name, rest) = match list.find([',', '=']) {
                Some(at) => list.split_at(at),
                None => (list, ""),
            };
            if let Some(rest) = rest.strip_prefix('=') {
                let (value, rest) = split_value(rest);
                let name = name.trim();
                if !name.is_empty() {
                    pairs.push((name, value));
                }
                list = rest;
            } else {
                // A bare word or an empty entry has no value to keep.
                list = rest.strip_prefix(',').unwrap_or(rest);
            }
        }
        Self(pairs)
    }

    /// Names are case-sensitive (RFC 8216 §4.2); the first of duplicates
    /// wins.
    fn get(&self, name: &str) -> Option<&'a str> {
        self.0
            .iter()
            .find(|(candidate, _)| *candidate == name)
            .map(|&(_, value)| value)
    }

    /// A non-empty value as an owned string.
    fn text(&self, name: &str) -> Option<String> {
        self.get(name)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    }

    fn number(&self, name: &str) -> Option<u64> {
        self.get(name)?.parse().ok()
    }
}

/// Splits `value,rest` (or `"quoted, value",rest`) at the comma that ends
/// the value. An unterminated quote runs to the end of the line.
fn split_value(s: &str) -> (&str, &str) {
    let s = s.trim_start();
    if let Some(quoted) = s.strip_prefix('"') {
        return match quoted.split_once('"') {
            // Anything between the closing quote and the next comma is junk.
            Some((value, rest)) => (value, rest.split_once(',').map_or("", |(_, rest)| rest)),
            None => (quoted, ""),
        };
    }
    match s.split_once(',') {
        Some((value, rest)) => (value.trim_end(), rest),
        None => (s.trim_end(), ""),
    }
}

fn invalid(reason: &str) -> DownloadError {
    DownloadError::InvalidManifest(reason.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Url {
        Url::parse("https://cdn.example.com/video/master.m3u8").unwrap()
    }

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    fn master(text: &str) -> MasterPlaylist {
        match parse(text, &base()) {
            Ok(Playlist::Master(m)) => m,
            other => panic!("expected a master playlist, got {other:?}"),
        }
    }

    fn media(text: &str) -> MediaPlaylist {
        match parse(text, &base()) {
            Ok(Playlist::Media(m)) => m,
            other => panic!("expected a media playlist, got {other:?}"),
        }
    }

    fn error(text: &str) -> DownloadError {
        match parse(text, &base()) {
            Err(err) => err,
            Ok(playlist) => panic!("expected an error, got {playlist:?}"),
        }
    }

    fn assert_invalid(text: &str) {
        let err = error(text);
        assert!(
            matches!(err, DownloadError::InvalidManifest(_)),
            "expected InvalidManifest, got {err:?}"
        );
    }

    fn iv_of(n: u128) -> [u8; 16] {
        n.to_be_bytes()
    }

    #[test]
    fn attributes_keep_quoted_commas_and_equals() {
        let attrs = Attributes::parse(
            r#"BANDWIDTH=1280000,CODECS="avc1.64001f,mp4a.40.2",URI="k?a=1,b=2",RESOLUTION=640x360"#,
        );
        assert_eq!(attrs.get("BANDWIDTH"), Some("1280000"));
        assert_eq!(attrs.get("CODECS"), Some("avc1.64001f,mp4a.40.2"));
        assert_eq!(attrs.get("URI"), Some("k?a=1,b=2"));
        assert_eq!(attrs.get("RESOLUTION"), Some("640x360"));
        assert_eq!(attrs.0.len(), 4);
    }

    #[test]
    fn attributes_are_case_sensitive_and_first_wins() {
        let attrs = Attributes::parse("METHOD=NONE,method=AES-128,METHOD=AES-128");
        assert_eq!(attrs.get("METHOD"), Some("NONE"));
        assert_eq!(attrs.get("method"), Some("AES-128"));
        assert_eq!(attrs.get("Method"), None);
    }

    #[test]
    fn attributes_tolerate_junk() {
        let attrs = Attributes::parse(r#" A = 1 , ,BARE, B="x" junk, C=,D="",E="open, ended"#);
        assert_eq!(attrs.get("A"), Some("1"));
        assert_eq!(attrs.get("BARE"), None);
        assert_eq!(attrs.get("B"), Some("x"));
        assert_eq!(attrs.get("C"), Some(""));
        assert_eq!(attrs.get("D"), Some(""));
        assert_eq!(attrs.text("D"), None);
        assert_eq!(attrs.get("E"), Some("open, ended"));
        assert_eq!(attrs.0.len(), 5);

        assert!(Attributes::parse("").0.is_empty());
        assert!(Attributes::parse(",,,").0.is_empty());
        assert!(Attributes::parse("=1,=\"x\"").0.is_empty());
        assert_eq!(Attributes::parse("X=1,").get("X"), Some("1"));
        assert_eq!(Attributes::parse("X==1").get("X"), Some("=1"));
    }

    const MASTER: &str = r#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-INDEPENDENT-SEGMENTS
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aac",NAME="English",LANGUAGE="en",DEFAULT=YES,AUTOSELECT=YES,CHANNELS="2",URI="audio/en.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="aac",NAME="Deutsch",LANGUAGE="de",DEFAULT=NO,AUTOSELECT=YES,URI="https://audio.example.org/de.m3u8"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID="muxed",NAME="Main",DEFAULT=YES
#EXT-X-MEDIA:TYPE=SUBTITLES,GROUP-ID="subs",NAME="English",URI="subs/en.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=5000000,AVERAGE-BANDWIDTH=4500000,RESOLUTION=1920x1080,FRAME-RATE=29.970,CODECS="avc1.640028,mp4a.40.2",AUDIO="aac"
1080p/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,CODECS="avc1.4d401f,mp4a.40.2",AUDIO="aac"
# a comment between the tag and its URI
/abs/720p.m3u8
#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=100000,URI="iframes.m3u8"
#EXT-X-STREAM-INF:BANDWIDTH=800000,AUDIO="muxed"
https://other.example.org/low.m3u8
"#;

    #[test]
    fn master_lists_variants_and_audio() {
        let m = master(MASTER);
        assert_eq!(
            m.variants,
            vec![
                Variant {
                    uri: url("https://cdn.example.com/video/1080p/index.m3u8"),
                    bandwidth: Some(5_000_000),
                    average_bandwidth: Some(4_500_000),
                    resolution: Some((1920, 1080)),
                    codecs: Some("avc1.640028,mp4a.40.2".into()),
                    frame_rate: Some("29.970".into()),
                    audio_group: Some("aac".into()),
                },
                Variant {
                    uri: url("https://cdn.example.com/abs/720p.m3u8"),
                    bandwidth: Some(2_500_000),
                    average_bandwidth: None,
                    resolution: Some((1280, 720)),
                    codecs: Some("avc1.4d401f,mp4a.40.2".into()),
                    frame_rate: None,
                    audio_group: Some("aac".into()),
                },
                Variant {
                    uri: url("https://other.example.org/low.m3u8"),
                    bandwidth: Some(800_000),
                    average_bandwidth: None,
                    resolution: None,
                    codecs: None,
                    frame_rate: None,
                    audio_group: Some("muxed".into()),
                },
            ]
        );
        assert_eq!(
            m.audio,
            vec![
                Rendition {
                    group_id: "aac".into(),
                    name: Some("English".into()),
                    language: Some("en".into()),
                    uri: Some(url("https://cdn.example.com/video/audio/en.m3u8")),
                    default: true,
                    autoselect: true,
                    channels: Some("2".into()),
                },
                Rendition {
                    group_id: "aac".into(),
                    name: Some("Deutsch".into()),
                    language: Some("de".into()),
                    uri: Some(url("https://audio.example.org/de.m3u8")),
                    default: false,
                    autoselect: true,
                    channels: None,
                },
                Rendition {
                    group_id: "muxed".into(),
                    name: Some("Main".into()),
                    language: None,
                    uri: None,
                    default: true,
                    autoselect: false,
                    channels: None,
                },
            ]
        );
    }

    #[test]
    fn uris_resolve_against_the_playlist() {
        let m = master(
            "#EXTM3U\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1\nrel.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=2\n../up/x.m3u8?token=a%2Fb\n\
             #EXT-X-STREAM-INF:BANDWIDTH=3\n/root.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=4\n//mirror.example.net/y.m3u8\n\
             #EXT-X-STREAM-INF:BANDWIDTH=5\nHTTP://Upper.Example.org/z.m3u8\n",
        );
        let uris: Vec<&str> = m.variants.iter().map(|v| v.uri.as_str()).collect();
        assert_eq!(
            uris,
            [
                "https://cdn.example.com/video/rel.m3u8",
                "https://cdn.example.com/up/x.m3u8?token=a%2Fb",
                "https://cdn.example.com/root.m3u8",
                "https://mirror.example.net/y.m3u8",
                "http://upper.example.org/z.m3u8",
            ]
        );
    }

    #[test]
    fn non_http_uris_are_refused() {
        assert_invalid("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nfile:///etc/passwd\n");
        assert_invalid(
            "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"data:text/plain;base64,AAAA\"\n\
             #EXTINF:4,\na.ts\n",
        );
        assert_invalid("#EXTM3U\n#EXT-X-ENDLIST\n#EXTINF:4,\nftp://h/a.ts\n");
        assert_invalid(
            "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"a\",URI=\"\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n",
        );
    }

    #[test]
    fn broken_masters_are_invalid() {
        // A variant without a URI, at the end or before the next variant.
        assert_invalid("#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\n");
        assert_invalid(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\n#EXT-X-STREAM-INF:BANDWIDTH=2\nb.m3u8\n",
        );
        // Audio without a group cannot be matched to a variant.
        assert_invalid(
            "#EXTM3U\n#EXT-X-MEDIA:TYPE=AUDIO,NAME=\"x\",URI=\"a.m3u8\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n",
        );
        // Only I-frame streams: nothing to download.
        assert_invalid("#EXTM3U\n#EXT-X-I-FRAME-STREAM-INF:BANDWIDTH=1,URI=\"i.m3u8\"\n");
        // Variants and segments in one file.
        assert_invalid(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n",
        );
    }

    #[test]
    fn plain_media_playlist() {
        let m = media(
            "#EXTM3U\n#EXT-X-VERSION:3\n#EXT-X-TARGETDURATION:10\n\
             #EXT-X-PROGRAM-DATE-TIME:2026-01-01T00:00:00Z\n\
             #EXTINF:9.009,\nseg0.ts\n\
             #EXT-X-DISCONTINUITY\n\
             #EXTINF:5,title, with comma\nhttps://other.example.org/seg1.ts\n\
             #EXTINF:2.0006\nseg2.ts\n\
             #EXT-X-ENDLIST\n",
        );
        assert_eq!(m.target_duration, Some(10));
        assert_eq!(m.media_sequence, 0);
        assert_eq!(m.init, None);
        assert_eq!(
            m.segments,
            vec![
                Segment {
                    duration_ms: 9009,
                    ..Segment::new(url("https://cdn.example.com/video/seg0.ts"))
                },
                Segment {
                    duration_ms: 5000,
                    ..Segment::new(url("https://other.example.org/seg1.ts"))
                },
                Segment {
                    duration_ms: 2001,
                    ..Segment::new(url("https://cdn.example.com/video/seg2.ts"))
                },
            ]
        );
    }

    #[test]
    fn aes_128_with_explicit_iv() {
        let m = media(
            "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-MEDIA-SEQUENCE:5\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"keys/k1.bin?t=1\",IV=0x000102030405060708090A0B0C0D0e0f\n\
             #EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"https://keys.example.org/k2\",KEYFORMAT=\"identity\",IV=0X1F\n\
             #EXTINF:4,\nc.ts\n",
        );
        let k1 = KeyRef {
            uri: url("https://cdn.example.com/video/keys/k1.bin?t=1"),
            iv: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
        };
        let k2 = KeyRef {
            uri: url("https://keys.example.org/k2"),
            iv: iv_of(0x1f),
        };
        let keys: Vec<_> = m.segments.iter().map(|s| s.key.clone()).collect();
        assert_eq!(keys, vec![Some(k1.clone()), Some(k1), Some(k2)]);
    }

    #[test]
    fn aes_128_iv_defaults_to_the_sequence_number() {
        let m = media(
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:258\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n\
             #EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n#EXT-X-ENDLIST\n",
        );
        assert_eq!(m.media_sequence, 258);
        let ivs: Vec<_> = m
            .segments
            .iter()
            .map(|s| s.key.as_ref().unwrap().iv)
            .collect();
        assert_eq!(ivs[0][14..], [1, 2]);
        assert_eq!(ivs, vec![iv_of(258), iv_of(259)]);

        let big = media(&format!(
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:{}\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n\
             #EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n#EXT-X-ENDLIST\n",
            u64::MAX
        ));
        let ivs: Vec<_> = big
            .segments
            .iter()
            .map(|s| s.key.as_ref().unwrap().iv)
            .collect();
        assert_eq!(
            ivs,
            vec![iv_of(u128::from(u64::MAX)), iv_of(u128::from(u64::MAX) + 1)]
        );
    }

    #[test]
    fn method_none_clears_the_key() {
        let m = media(
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXTINF:4,\na.ts\n\
             #EXT-X-KEY:METHOD=NONE\n#EXTINF:4,\nb.ts\n#EXTINF:4,\nc.ts\n#EXT-X-ENDLIST\n",
        );
        let keyed: Vec<bool> = m.segments.iter().map(|s| s.key.is_some()).collect();
        assert_eq!(keyed, [true, false, false]);
    }

    #[test]
    fn broken_keys_are_invalid() {
        let with_key = |key: &str| format!("#EXTM3U\n{key}\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
        assert_invalid(&with_key("#EXT-X-KEY:METHOD=AES-128"));
        assert_invalid(&with_key("#EXT-X-KEY:URI=\"k\""));
        for iv in [
            "0xZZ",
            "00ff",
            "0x",
            "0x+1",
            &format!("0x{}", "1".repeat(33)),
        ] {
            assert_invalid(&with_key(&format!(
                "#EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV={iv}"
            )));
        }
        let max = media(&with_key(&format!(
            "#EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV=0x{}",
            "f".repeat(32)
        )));
        assert_eq!(max.segments[0].key.as_ref().unwrap().iv, [0xff; 16]);
    }

    #[test]
    fn byte_ranges_with_and_without_offsets() {
        let m = media(
            "#EXTM3U\n#EXT-X-ENDLIST\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:1000@500\nall.ts\n\
             #EXT-X-BYTERANGE:2000\n#EXTINF:4,\nall.ts\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:300\nall.ts\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:10@0\nother.ts\n",
        );
        let ranges: Vec<_> = m.segments.iter().map(|s| s.range).collect();
        assert_eq!(
            ranges,
            vec![
                Some(ByteSpan {
                    offset: 500,
                    length: 1000
                }),
                Some(ByteSpan {
                    offset: 1500,
                    length: 2000
                }),
                Some(ByteSpan {
                    offset: 3500,
                    length: 300
                }),
                Some(ByteSpan {
                    offset: 0,
                    length: 10
                }),
            ]
        );
    }

    #[test]
    fn byte_ranges_that_cannot_be_placed_are_invalid() {
        // Nothing before it.
        assert_invalid("#EXTM3U\n#EXTINF:4,\n#EXT-X-BYTERANGE:100\na.ts\n#EXT-X-ENDLIST\n");
        // The previous range is of another resource.
        assert_invalid(
            "#EXTM3U\n#EXTINF:4,\n#EXT-X-BYTERANGE:100@0\na.ts\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:100\nb.ts\n#EXT-X-ENDLIST\n",
        );
        // The previous segment is a whole resource.
        assert_invalid(
            "#EXTM3U\n#EXTINF:4,\na.ts\n#EXTINF:4,\n#EXT-X-BYTERANGE:100\na.ts\n#EXT-X-ENDLIST\n",
        );
        for range in [
            "abc",
            "0@0",
            "-1@0",
            "1@x",
            "18446744073709551616",
            "2@18446744073709551615",
        ] {
            assert_invalid(&format!(
                "#EXTM3U\n#EXTINF:4,\n#EXT-X-BYTERANGE:{range}\na.ts\n#EXT-X-ENDLIST\n"
            ));
        }
    }

    #[test]
    fn map_with_byte_range_and_key() {
        let m = media(
            "#EXTM3U\n#EXT-X-ENDLIST\n\
             #EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n\
             #EXT-X-MAP:URI=\"main.mp4\",BYTERANGE=\"720@0\"\n\
             #EXT-X-MEDIA-SEQUENCE:9\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:1000@720\nmain.mp4\n\
             #EXT-X-MAP:URI=\"main.mp4\",BYTERANGE=\"720@0\"\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:1000\nmain.mp4\n",
        );
        let key = |seq| KeyRef {
            uri: url("https://cdn.example.com/video/k"),
            iv: iv_of(seq),
        };
        let file = url("https://cdn.example.com/video/main.mp4");
        // The map takes the sequence number of the segment after it, even
        // when `#EXT-X-MEDIA-SEQUENCE` is written after the map.
        assert_eq!(
            m.init,
            Some(Segment {
                range: Some(ByteSpan {
                    offset: 0,
                    length: 720
                }),
                key: Some(key(9)),
                ..Segment::new(file.clone())
            })
        );
        assert_eq!(
            m.segments[1].range,
            Some(ByteSpan {
                offset: 1720,
                length: 1000
            })
        );
        assert_eq!(m.segments[1].key, Some(key(10)));

        let plain = media(
            "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\",BYTERANGE=\"500\"\n#EXTINF:4,\na.m4s\n#EXT-X-ENDLIST\n",
        );
        let init = plain.init.unwrap();
        assert_eq!(
            init.range,
            Some(ByteSpan {
                offset: 0,
                length: 500
            })
        );
        assert_eq!(init.key, None);
        assert_eq!(init.duration_ms, 0);

        let explicit_iv = media(
            "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV=0x2A\n#EXT-X-MAP:URI=\"init.mp4\"\n\
             #EXTINF:4,\na.m4s\n#EXT-X-ENDLIST\n",
        );
        assert_eq!(explicit_iv.init.unwrap().key.unwrap().iv, iv_of(42));
    }

    #[test]
    fn a_changing_map_is_unsupported() {
        for second in ["URI=\"other.mp4\"", "URI=\"init.mp4\",BYTERANGE=\"10@0\""] {
            let err = error(&format!(
                "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:4,\na.m4s\n\
                 #EXT-X-DISCONTINUITY\n#EXT-X-MAP:{second}\n#EXTINF:4,\nb.m4s\n#EXT-X-ENDLIST\n"
            ));
            assert!(
                matches!(&err, DownloadError::UnsupportedMedia(m) if m.contains("initialization")),
                "{err:?}"
            );
        }
        assert_invalid(
            "#EXTM3U\n#EXT-X-MAP:BYTERANGE=\"1@0\"\n#EXTINF:4,\na.m4s\n#EXT-X-ENDLIST\n",
        );
    }

    #[test]
    fn gaps_are_left_out_but_keep_their_place() {
        let m = media(
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:3\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:100@0\nall.ts\n\
             #EXT-X-GAP\n#EXTINF:4,\n#EXT-X-BYTERANGE:100\nall.ts\n\
             #EXTINF:4,\n#EXT-X-BYTERANGE:100\nall.ts\n#EXT-X-ENDLIST\n",
        );
        assert_eq!(m.segments.len(), 2);
        assert_eq!(
            m.segments[1].range,
            Some(ByteSpan {
                offset: 200,
                length: 100
            })
        );
        assert_eq!(m.segments[1].key.as_ref().unwrap().iv, iv_of(5));
    }

    #[test]
    fn live_playlists_are_refused() {
        let live = "#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXTINF:4,\na.ts\n#EXTINF:4,\nb.ts\n";
        assert!(matches!(error(live), DownloadError::LiveStream));
        let event = "#EXTM3U\n#EXT-X-PLAYLIST-TYPE:EVENT\n#EXTINF:4,\na.ts\n";
        assert!(matches!(error(event), DownloadError::LiveStream));
        // A live window that starts mid-way through a byte-range run.
        let window = "#EXTM3U\n#EXTINF:4,\n#EXT-X-BYTERANGE:100\na.ts\n";
        assert!(matches!(error(window), DownloadError::LiveStream));
        assert!(matches!(error("#EXTM3U\n"), DownloadError::LiveStream));

        assert_eq!(media(&format!("{event}#EXT-X-ENDLIST\n")).segments.len(), 1);
        let vod = media("#EXTM3U\n#EXT-X-PLAYLIST-TYPE:VOD\n#EXTINF:4,\na.ts\n");
        assert_eq!(vod.segments.len(), 1);
    }

    #[test]
    fn drm_is_refused() {
        let protected = |text: &str| {
            let err = error(text);
            assert!(matches!(err, DownloadError::Protected), "{err:?}");
        };
        let media_with = |key: &str| format!("#EXTM3U\n{key}\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
        protected(&media_with("#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"k\""));
        protected(&media_with("#EXT-X-KEY:METHOD=SAMPLE-AES-CTR,URI=\"k\""));
        protected(&media_with("#EXT-X-KEY:METHOD=SAMPLE-AES-CENC,URI=\"k\""));
        protected(&media_with("#EXT-X-KEY:METHOD=ISO-23001-7,URI=\"k\""));
        protected(&media_with("#EXT-X-KEY:METHOD=AES-256,URI=\"k\""));
        protected(&media_with(
            "#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://key-id\",KEYFORMAT=\"com.apple.streamingkeydelivery\",KEYFORMATVERSIONS=\"1\"",
        ));
        // Plain AES-128 handed to a FairPlay key server is still FairPlay.
        protected(&media_with(
            "#EXT-X-KEY:METHOD=AES-128,URI=\"skd://key-id\",KEYFORMAT=\"com.apple.streamingkeydelivery\"",
        ));
        // A clear key next to a DRM key for the same segments.
        protected(&media_with(
            "#EXT-X-KEY:METHOD=AES-128,URI=\"k\"\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"data:x\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"",
        ));
        // Widevine announced in the master playlist.
        protected(
            "#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES-CTR,URI=\"data:text/plain;base64,AAAA\",KEYFORMAT=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\",KEYFORMATVERSIONS=\"1\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n",
        );
        protected(
            "#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"k\",KEYFORMAT=\"com.microsoft.playready\"\n\
             #EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n",
        );
        // Protected wins over live and over other mistakes.
        protected("#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"k\"\n#EXTINF:4,\na.ts\n");
        protected("#EXTM3U\n#EXT-X-KEY:METHOD=SAMPLE-AES\n#EXT-X-STREAM-INF:BANDWIDTH=1\n");

        // Clear session keys are fine.
        let m = master(
            "#EXTM3U\n#EXT-X-SESSION-KEY:METHOD=AES-128,URI=\"k\",KEYFORMAT=\"identity\"\n\
             #EXT-X-SESSION-KEY:METHOD=NONE\n#EXT-X-STREAM-INF:BANDWIDTH=1\nv.m3u8\n",
        );
        assert_eq!(m.variants.len(), 1);
    }

    #[test]
    fn not_a_playlist() {
        for text in [
            "",
            "\n\n  \n",
            "<!doctype html><html></html>",
            "#EXTINF:4,\na.ts\n",
            "#EXTM3U8\n#EXT-X-ENDLIST\n",
            "\u{feff}",
        ] {
            let err = error(text);
            assert!(
                matches!(&err, DownloadError::InvalidManifest(m) if m == "this is not an HLS playlist"),
                "{text:?}: {err:?}"
            );
        }
    }

    #[test]
    fn bom_crlf_and_blank_lines() {
        let m = media(
            "\u{feff}\r\n  #EXTM3U  \r\n\r\n#EXT-X-TARGETDURATION:6\r\n#EXTINF:6.0,\r\n\r\n  seg.ts  \r\n#EXT-X-ENDLIST\r\n",
        );
        assert_eq!(m.target_duration, Some(6));
        assert_eq!(
            m.segments,
            vec![Segment {
                duration_ms: 6000,
                ..Segment::new(url("https://cdn.example.com/video/seg.ts"))
            }]
        );
    }

    #[test]
    fn finished_playlists_without_segments_are_invalid() {
        assert_invalid("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-ENDLIST\n");
        assert_invalid("#EXTM3U\n#EXT-X-GAP\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
    }

    #[test]
    fn malformed_media_playlists_are_invalid() {
        // A URI without `#EXTINF`, and an `#EXTINF` without a URI.
        assert_invalid("#EXTM3U\na.ts\n#EXT-X-ENDLIST\n");
        assert_invalid("#EXTM3U\n#EXTINF:4,\na.ts\n#EXTINF:4,\n#EXT-X-ENDLIST\n");
        for duration in ["abc", "-1", "NaN", "inf", "1e300", ""] {
            assert_invalid(&format!(
                "#EXTM3U\n#EXTINF:{duration},\na.ts\n#EXT-X-ENDLIST\n"
            ));
        }
        assert_invalid(
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:99999999999999999999\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n",
        );
        assert_invalid("#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:x\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
        // The sequence number cannot change once segments have used it.
        assert_invalid("#EXTM3U\n#EXTINF:4,\na.ts\n#EXT-X-MEDIA-SEQUENCE:5\n#EXT-X-ENDLIST\n");
        // Informational junk is ignored instead.
        let m = media("#EXTM3U\n#EXT-X-TARGETDURATION:soon\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
        assert_eq!(m.target_duration, None);
    }

    #[test]
    fn too_many_segments_are_unsupported() {
        let mut text = String::from("#EXTM3U\n#EXT-X-ENDLIST\n");
        for _ in 0..=MAX_SEGMENTS {
            text.push_str("#EXTINF:1,\na.ts\n");
        }
        let err = error(&text);
        assert!(
            matches!(&err, DownloadError::UnsupportedMedia(m) if m.contains("too many")),
            "{err:?}"
        );
    }

    #[test]
    fn never_panics_on_truncated_input() {
        let samples = [
            MASTER,
            "#EXTM3U\n#EXT-X-MEDIA-SEQUENCE:7\n#EXT-X-KEY:METHOD=AES-128,URI=\"k\",IV=0xABC\n\
             #EXT-X-MAP:URI=\"i.mp4\",BYTERANGE=\"10@0\"\n#EXTINF:4,t\n#EXT-X-BYTERANGE:5@10\na.m4s\n\
             #EXT-X-GAP\n#EXTINF:4,\n#EXT-X-BYTERANGE:5\na.m4s\n#EXT-X-ENDLIST\n",
        ];
        for sample in samples {
            for (end, _) in sample.char_indices() {
                let _ = parse(&sample[..end], &base());
            }
        }
    }
}
