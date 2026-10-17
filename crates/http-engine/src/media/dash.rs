//! MPEG-DASH manifests (MPD): adaptation sets of representations, each
//! addressed by a segment template, a segment list or one whole file.
//!
//! Only static (on-demand) manifests with a single period are read. The text
//! comes from an arbitrary server, so every number taken from it goes through
//! checked arithmetic and every list it can make long is capped.

use std::str::FromStr;
use std::time::Duration;

use roxmltree::{Document, Node, ParsingOptions};
use url::Url;

use super::{ByteSpan, Segment};
use crate::DownloadError;

/// Per representation. Hours of one-second fragments stay well below it; a
/// manifest above it is broken or trying to exhaust memory.
const MAX_SEGMENTS: u64 = 100_000;
/// Across representations, for the same reason: hundreds of representations
/// just under [`MAX_SEGMENTS`] would still add up to gigabytes.
const MAX_TOTAL_SEGMENTS: usize = 1_000_000;
/// A `u64` has at most 20 digits; a wider `%0Nd` would only inflate URLs.
const MAX_TEMPLATE_WIDTH: usize = 32;
const NANOS_PER_SEC: u128 = 1_000_000_000;
const ROLE_SCHEME: &str = "urn:mpeg:dash:role:2011";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// `mediaPresentationDuration` (or the Period's `duration`).
    pub duration_ms: Option<u64>,
    /// Every representation of the (single) period, in document order.
    pub representations: Vec<Representation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    Video,
    Audio,
    Text,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Representation {
    pub id: String,
    pub kind: ContentKind,
    pub mime_type: Option<String>,
    pub codecs: Option<String>,
    pub bandwidth: Option<u64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_rate: Option<String>,
    pub language: Option<String>,
    /// `Label` of the representation or its adaptation set.
    pub label: Option<String>,
    /// The adaptation set has `Role` `urn:mpeg:dash:role:2011` with value `main`.
    pub main: bool,
    pub init: Option<Segment>,
    pub segments: Vec<Segment>,
}

/// Parse an MPD fetched from `mpd_url` (relative URLs resolve against it).
pub fn parse(text: &str, mpd_url: &Url) -> Result<Manifest, DownloadError> {
    // Without a DTD there are no entities, so nothing can expand.
    let options = ParsingOptions {
        allow_dtd: false,
        ..Default::default()
    };
    let doc = Document::parse_with_options(text, options)
        .map_err(|e| invalid(format!("the XML cannot be read ({e})")))?;
    let mpd = doc.root_element();
    if mpd.tag_name().name() != "MPD" {
        return Err(invalid("this is not a DASH manifest"));
    }
    // Protected tracks must stop the download, not quietly drop out of the
    // list, so the whole document is checked before anything is built.
    if doc.descendants().any(|n| is_named(n, "ContentProtection")) {
        return Err(DownloadError::Protected);
    }
    if mpd.attribute("type").map(str::trim) == Some("dynamic") {
        return Err(DownloadError::LiveStream);
    }

    let mut periods = elements(mpd, "Period");
    let period = periods
        .next()
        .ok_or_else(|| invalid("the manifest has no Period"))?;
    if periods.next().is_some() {
        return Err(DownloadError::UnsupportedMedia(
            "manifests with several periods are not supported yet".into(),
        ));
    }
    let presentation = duration_attribute(mpd, "mediaPresentationDuration")?;
    let period_duration = duration_attribute(period, "duration")?;

    let base = with_base_url(mpd_url, mpd)?;
    let base = with_base_url(&base, period)?;
    let mut representations = Vec::new();
    let mut total_segments = 0usize;
    for (set_index, set) in elements(period, "AdaptationSet").enumerate() {
        let parent = Parent {
            period,
            set,
            base: with_base_url(&base, set)?,
            period_len: period_duration.or(presentation),
        };
        for (rep_index, rep) in elements(set, "Representation").enumerate() {
            let rep = parent.representation(rep, (set_index, rep_index))?;
            total_segments = total_segments.saturating_add(rep.segments.len());
            if total_segments > MAX_TOTAL_SEGMENTS {
                return Err(too_many_segments());
            }
            representations.push(rep);
        }
    }
    if representations.is_empty() {
        return Err(invalid("the manifest has no representations"));
    }
    Ok(Manifest {
        duration_ms: presentation.or(period_duration).map(millis),
        representations,
    })
}

/// An adaptation set and what its representations inherit from above it.
struct Parent<'a, 'i> {
    period: Node<'a, 'i>,
    set: Node<'a, 'i>,
    base: Url,
    period_len: Option<Duration>,
}

impl<'a, 'i> Parent<'a, 'i> {
    fn representation(
        &self,
        rep: Node<'a, 'i>,
        (set_index, rep_index): (usize, usize),
    ) -> Result<Representation, DownloadError> {
        let set = self.set;
        let inherited = |name: &str| rep.attribute(name).or_else(|| set.attribute(name));
        // `$RepresentationID$` needs some id, and it must stay stable when
        // the same manifest is fetched again.
        let id = match rep.attribute("id") {
            Some(id) => id.to_string(),
            None => format!("{set_index}-{rep_index}"),
        };
        let codecs = text_value(inherited("codecs"));
        let bandwidth = parse_number("bandwidth", rep.attribute("bandwidth"))?;
        let base = with_base_url(&self.base, rep)?;
        let target = Target {
            representation_id: &id,
            bandwidth,
            base: &base,
            period_len: self.period_len,
        };
        let (init, segments) = target.segments([rep, set, self.period])?;
        if segments.is_empty() {
            return Err(invalid("a representation has no segments"));
        }
        Ok(Representation {
            kind: content_kind(rep, set, codecs.as_deref()),
            mime_type: text_value(inherited("mimeType")),
            codecs,
            bandwidth,
            width: parse_number("width", inherited("width"))?,
            height: parse_number("height", inherited("height"))?,
            frame_rate: text_value(inherited("frameRate")),
            language: text_value(set.attribute("lang")),
            label: label(rep).or_else(|| label(set)),
            main: elements(set, "Role").any(|role| {
                role.attribute("schemeIdUri").map(str::trim) == Some(ROLE_SCHEME)
                    && role.attribute("value").map(str::trim) == Some("main")
            }),
            init,
            segments,
            id,
        })
    }
}

/// What the URLs of one representation are built from.
struct Target<'a> {
    representation_id: &'a str,
    bandwidth: Option<u64>,
    base: &'a Url,
    period_len: Option<Duration>,
}

/// One media segment of a template before its URL is built.
struct Slot {
    number: u64,
    time: Option<u64>,
    ticks: u64,
}

type Addressed = (Option<Segment>, Vec<Segment>);

impl Target<'_> {
    /// `levels` is Representation, AdaptationSet, Period. The most specific
    /// level that names an addressing scheme picks it; the same element on
    /// the levels above still fills in what it leaves out.
    fn segments(&self, levels: [Node<'_, '_>; 3]) -> Result<Addressed, DownloadError> {
        for level in levels {
            if child(level, "SegmentTemplate").is_some() {
                return self.template_segments(&Layered::new(levels, "SegmentTemplate"));
            }
            if child(level, "SegmentList").is_some() {
                return self.list_segments(&Layered::new(levels, "SegmentList"));
            }
            if child(level, "SegmentBase").is_some() {
                break;
            }
        }
        self.whole_file()
    }

    fn template_segments(&self, template: &Layered<'_, '_>) -> Result<Addressed, DownloadError> {
        let media = template
            .attribute("media")
            .ok_or_else(|| invalid("a SegmentTemplate has no `media`"))?;
        let timescale = timescale(template)?;
        let start_number = template.number("startNumber")?.unwrap_or(1);
        let init = match template.attribute("initialization") {
            Some(pattern) => Some(Segment::new(
                self.resolve(&self.fill(pattern, None, None)?)?,
            )),
            None => template
                .child("Initialization")
                .map(|node| self.initialization(node))
                .transpose()?,
        };
        let slots = match template.child("SegmentTimeline") {
            Some(timeline) => {
                let offset = template.number("presentationTimeOffset")?.unwrap_or(0);
                self.timeline_slots(timeline, start_number, timescale, offset)?
            }
            None => self.fixed_slots(template, start_number, timescale)?,
        };
        let segments = slots
            .into_iter()
            .map(|slot| {
                let url = self.resolve(&self.fill(media, Some(slot.number), slot.time)?)?;
                Ok(Segment {
                    url,
                    range: None,
                    key: None,
                    duration_ms: ticks_to_ms(slot.ticks, timescale),
                })
            })
            .collect::<Result<_, DownloadError>>()?;
        Ok((init, segments))
    }

    /// `<S t d r>` entries. `t` continues from the previous segment when
    /// absent, and `r="-1"` repeats up to the next entry's `t` or, for the
    /// last entry, to the end of the period.
    fn timeline_slots(
        &self,
        timeline: Node<'_, '_>,
        start_number: u64,
        timescale: u64,
        offset: u64,
    ) -> Result<Vec<Slot>, DownloadError> {
        let entries: Vec<_> = elements(timeline, "S").collect();
        let mut slots = Vec::new();
        let mut time = 0u64;
        for (index, entry) in entries.iter().enumerate() {
            if let Some(t) = parse_number("t", entry.attribute("t"))? {
                time = t;
            }
            let ticks: u64 = parse_number("d", entry.attribute("d"))?
                .filter(|d| *d > 0)
                .ok_or_else(|| invalid("a SegmentTimeline entry has no positive `d`"))?;
            let repeat: i64 = parse_number("r", entry.attribute("r"))?.unwrap_or(0);
            let count = if repeat == -1 {
                let next_start = match entries.get(index + 1) {
                    Some(next) => parse_number::<u64>("t", next.attribute("t"))?,
                    None => None,
                };
                let end = match next_start {
                    Some(t) => t,
                    None => self.period_end(timescale, offset)?,
                };
                end.saturating_sub(time).div_ceil(ticks)
            } else {
                u64::try_from(repeat)
                    .ok()
                    .and_then(|r| r.checked_add(1))
                    .ok_or_else(|| invalid("a SegmentTimeline entry has an invalid `r`"))?
            };
            // Checked before the loop: `r` alone can ask for billions.
            if count > MAX_SEGMENTS.saturating_sub(slots.len() as u64) {
                return Err(too_many_segments());
            }
            for _ in 0..count {
                let number = start_number
                    .checked_add(slots.len() as u64)
                    .ok_or_else(overflow)?;
                slots.push(Slot {
                    number,
                    time: Some(time),
                    ticks,
                });
                time = time.checked_add(ticks).ok_or_else(overflow)?;
            }
        }
        Ok(slots)
    }

    /// Segments of a fixed `duration`, numbered from `startNumber` up to
    /// `endNumber` or for as long as the period lasts.
    fn fixed_slots(
        &self,
        template: &Layered<'_, '_>,
        start_number: u64,
        timescale: u64,
    ) -> Result<Vec<Slot>, DownloadError> {
        let ticks: u64 = template
            .number("duration")?
            .filter(|d| *d > 0)
            .ok_or_else(|| {
                invalid("a SegmentTemplate has neither a SegmentTimeline nor a positive `duration`")
            })?;
        let count = match template.number::<u64>("endNumber")? {
            Some(end) => end
                .checked_sub(start_number)
                .and_then(|n| n.checked_add(1))
                .ok_or_else(|| invalid("`endNumber` is below `startNumber`"))?,
            None => {
                let len = self.period_len.ok_or_else(|| {
                    invalid("the number of segments is unknown: the manifest has no duration")
                })?;
                period_ticks(len, timescale)?.div_ceil(ticks)
            }
        };
        if count > MAX_SEGMENTS {
            return Err(too_many_segments());
        }
        (0..count)
            .map(|i| {
                Ok(Slot {
                    number: start_number.checked_add(i).ok_or_else(overflow)?,
                    time: None,
                    ticks,
                })
            })
            .collect()
    }

    /// End of the period on the media timeline, in `timescale` units.
    fn period_end(&self, timescale: u64, offset: u64) -> Result<u64, DownloadError> {
        let len = self.period_len.ok_or_else(|| {
            invalid("a SegmentTimeline repeats to the end of the period, but its length is unknown")
        })?;
        offset
            .checked_add(period_ticks(len, timescale)?)
            .ok_or_else(overflow)
    }

    fn list_segments(&self, list: &Layered<'_, '_>) -> Result<Addressed, DownloadError> {
        let timescale = timescale(list)?;
        let duration_ms = list
            .number("duration")?
            .map_or(0, |d| ticks_to_ms(d, timescale));
        let init = list
            .child("Initialization")
            .map(|node| self.initialization(node))
            .transpose()?;
        let urls = list.children("SegmentURL");
        if urls.len() as u64 > MAX_SEGMENTS {
            return Err(too_many_segments());
        }
        let segments = urls
            .into_iter()
            .map(|node| {
                let mut segment =
                    self.ranged(node.attribute("media"), node.attribute("mediaRange"))?;
                segment.duration_ms = duration_ms;
                Ok(segment)
            })
            .collect::<Result<_, DownloadError>>()?;
        Ok((init, segments))
    }

    /// `SegmentBase` or no addressing at all: the base URL is the whole track.
    fn whole_file(&self) -> Result<Addressed, DownloadError> {
        let mut file = Segment::new(web_url(self.base.clone())?);
        file.duration_ms = self.period_len.map_or(0, millis);
        Ok((None, vec![file]))
    }

    fn initialization(&self, node: Node<'_, '_>) -> Result<Segment, DownloadError> {
        self.ranged(node.attribute("sourceURL"), node.attribute("range"))
    }

    /// A URL (the base URL when there is none), optionally narrowed to a
    /// byte range of it.
    fn ranged(&self, url: Option<&str>, range: Option<&str>) -> Result<Segment, DownloadError> {
        let url = match url {
            Some(url) => self.resolve(url)?,
            None => web_url(self.base.clone())?,
        };
        Ok(Segment {
            url,
            range: range.map(parse_range).transpose()?,
            key: None,
            duration_ms: 0,
        })
    }

    fn fill(
        &self,
        pattern: &str,
        number: Option<u64>,
        time: Option<u64>,
    ) -> Result<String, DownloadError> {
        let vars = TemplateVars {
            representation_id: self.representation_id,
            bandwidth: self.bandwidth,
            number,
            time,
        };
        fill_template(pattern, &vars)
    }

    fn resolve(&self, reference: &str) -> Result<Url, DownloadError> {
        let url = self
            .base
            .join(reference.trim())
            .map_err(|_| invalid("a segment URL is not valid"))?;
        web_url(url)
    }
}

/// One addressing element as it appears on the Representation, the
/// AdaptationSet and the Period, most specific first. Each attribute or child
/// comes from the most specific level that has it.
struct Layered<'a, 'i>(Vec<Node<'a, 'i>>);

impl<'a, 'i> Layered<'a, 'i> {
    fn new(levels: [Node<'a, 'i>; 3], name: &str) -> Self {
        Self(
            levels
                .into_iter()
                .filter_map(|level| child(level, name))
                .collect(),
        )
    }

    fn attribute(&self, name: &str) -> Option<&'a str> {
        self.0.iter().find_map(|node| node.attribute(name))
    }

    fn number<T: FromStr>(&self, name: &str) -> Result<Option<T>, DownloadError> {
        parse_number(name, self.attribute(name))
    }

    fn child(&self, name: &str) -> Option<Node<'a, 'i>> {
        self.0.iter().find_map(|node| child(*node, name))
    }

    /// Every `name` child of the most specific level that has any: a list is
    /// replaced as a whole, never merged.
    fn children(&self, name: &str) -> Vec<Node<'a, 'i>> {
        self.0
            .iter()
            .map(|node| elements(*node, name).collect::<Vec<_>>())
            .find(|found| !found.is_empty())
            .unwrap_or_default()
    }
}

fn timescale(layered: &Layered<'_, '_>) -> Result<u64, DownloadError> {
    match layered.number("timescale")? {
        Some(0) => Err(invalid("`timescale` is 0")),
        other => Ok(other.unwrap_or(1)),
    }
}

/// Values the `$…$` identifiers of a SegmentTemplate stand for.
struct TemplateVars<'a> {
    representation_id: &'a str,
    bandwidth: Option<u64>,
    number: Option<u64>,
    time: Option<u64>,
}

/// Expand `$RepresentationID$`, `$Bandwidth$`, `$Number$`, `$Time$` (the
/// last three optionally with a `%0Nd` width) and `$$`.
fn fill_template(pattern: &str, vars: &TemplateVars<'_>) -> Result<String, DownloadError> {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;
    while let Some((literal, tail)) = rest.split_once('$') {
        out.push_str(literal);
        let (identifier, tail) = tail
            .split_once('$')
            .ok_or_else(|| invalid("a SegmentTemplate has an unclosed `$`"))?;
        rest = tail;
        let (name, format) = match identifier.split_once('%') {
            Some((name, format)) => (name, Some(format)),
            None => (identifier, None),
        };
        let value = match (name, format) {
            ("", None) => {
                out.push('$');
                continue;
            }
            ("RepresentationID", None) => {
                out.push_str(vars.representation_id);
                continue;
            }
            ("Bandwidth", _) => vars.bandwidth,
            ("Number", _) => vars.number,
            ("Time", _) => vars.time,
            _ => return Err(invalid("a SegmentTemplate uses an unknown `$` identifier")),
        };
        let value = value.ok_or_else(|| {
            invalid(format!(
                "a SegmentTemplate uses `${name}$` where it has no value"
            ))
        })?;
        let width = format.map(template_width).transpose()?.unwrap_or(0);
        out.push_str(&format!("{value:0width$}"));
    }
    out.push_str(rest);
    Ok(out)
}

/// `05d` (from `$Number%05d$`) is 5. Padding is always with zeros.
fn template_width(format: &str) -> Result<usize, DownloadError> {
    let bad = || invalid("a SegmentTemplate has an invalid `%0Nd` width");
    let digits = format.strip_suffix('d').ok_or_else(bad)?;
    if digits.is_empty() {
        return Ok(0);
    }
    if !is_digits(digits) {
        return Err(bad());
    }
    match digits.parse::<usize>() {
        Ok(width) if width <= MAX_TEMPLATE_WIDTH => Ok(width),
        _ => Err(bad()),
    }
}

/// The first source that names a kind wins: the representation's MIME type,
/// the adaptation set's, its `contentType`, its content components and last
/// the codecs. A source that names nothing known (`application/mp4` without
/// a subtitle codec, say) leaves the decision to the next one.
fn content_kind(rep: Node<'_, '_>, set: Node<'_, '_>, codecs: Option<&str>) -> ContentKind {
    let mimes = [rep.attribute("mimeType"), set.attribute("mimeType")]
        .into_iter()
        .flatten()
        .map(|mime| mime_kind(mime, codecs));
    let components = dominant(
        elements(set, "ContentComponent")
            .filter_map(|component| component.attribute("contentType"))
            .map(type_kind),
    );
    mimes
        .chain(set.attribute("contentType").map(type_kind))
        .chain([components, codecs.map_or(ContentKind::Other, codecs_kind)])
        .find(|kind| *kind != ContentKind::Other)
        .unwrap_or(ContentKind::Other)
}

fn mime_kind(mime: &str, codecs: Option<&str>) -> ContentKind {
    let essence = mime
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let subtitle_codecs =
        || codecs.is_some_and(|c| c.split(',').any(|c| codec_kind(c) == ContentKind::Text));
    match essence.split_once('/') {
        Some(("video", _)) => ContentKind::Video,
        Some(("audio", _)) => ContentKind::Audio,
        Some(("text", _)) | Some(("application", "ttml+xml")) => ContentKind::Text,
        Some(("application", "mp4")) if subtitle_codecs() => ContentKind::Text,
        _ => ContentKind::Other,
    }
}

fn type_kind(content_type: &str) -> ContentKind {
    match content_type.trim().to_ascii_lowercase().as_str() {
        "video" => ContentKind::Video,
        "audio" => ContentKind::Audio,
        "text" => ContentKind::Text,
        _ => ContentKind::Other,
    }
}

fn codecs_kind(codecs: &str) -> ContentKind {
    dominant(codecs.split(',').map(codec_kind))
}

/// `avc1.64001f` is video: only the family before the first dot matters.
fn codec_kind(codec: &str) -> ContentKind {
    let family = codec
        .trim()
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    match family.as_str() {
        "avc1" | "avc3" | "hvc1" | "hev1" | "vp8" | "vp9" | "vp09" | "av01" => ContentKind::Video,
        "mp4a" | "opus" | "vorbis" | "ac-3" | "ec-3" | "flac" | "mp3" => ContentKind::Audio,
        "stpp" | "wvtt" => ContentKind::Text,
        _ => ContentKind::Other,
    }
}

/// A muxed stream lists its video next to its audio, and is a video.
fn dominant(kinds: impl IntoIterator<Item = ContentKind>) -> ContentKind {
    let mut found = ContentKind::Other;
    for kind in kinds {
        if kind == ContentKind::Video {
            return kind;
        }
        if found == ContentKind::Other {
            found = kind;
        }
    }
    found
}

fn label(node: Node<'_, '_>) -> Option<String> {
    text_value(child(node, "Label").and_then(|label| label.text()))
}

/// `base` moved by the node's first `BaseURL`, which may be relative to it.
fn with_base_url(base: &Url, node: Node<'_, '_>) -> Result<Url, DownloadError> {
    match child(node, "BaseURL") {
        Some(base_url) => base
            .join(base_url.text().unwrap_or_default().trim())
            .map_err(|_| invalid("a BaseURL is not valid")),
        None => Ok(base.clone()),
    }
}

/// The manifest's author picks these URLs; only web requests may come of them.
fn web_url(url: Url) -> Result<Url, DownloadError> {
    match url.scheme() {
        "http" | "https" => Ok(url),
        _ => Err(invalid("segment URLs must use http or https")),
    }
}

/// ISO 8601 durations as MPDs write them: `P1DT2H3M4.5S`. Years and months
/// have no fixed length, so they are refused rather than guessed.
fn parse_duration(text: &str) -> Result<Duration, DownloadError> {
    let bad = || invalid("a duration is not valid");
    let mut rest = text.trim().strip_prefix('P').ok_or_else(bad)?;
    let mut nanos: u128 = 0;
    let mut in_time = false;
    // Units come in this order, each at most once.
    let mut last_unit = 0;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('T') {
            if in_time || after.is_empty() {
                return Err(bad());
            }
            in_time = true;
            rest = after;
            continue;
        }
        let split = rest
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
            .ok_or_else(bad)?;
        let (number, tail) = rest.split_at(split);
        let mut chars = tail.chars();
        let unit = chars.next().ok_or_else(bad)?;
        rest = chars.as_str();
        let (order, seconds): (u8, u128) = match (in_time, unit) {
            (false, 'D') => (1, 86_400),
            (true, 'H') => (2, 3_600),
            (true, 'M') => (3, 60),
            (true, 'S') => (4, 1),
            (false, 'Y' | 'M' | 'W') => {
                return Err(invalid(
                    "durations in years, months or weeks are not supported",
                ));
            }
            _ => return Err(bad()),
        };
        if order <= last_unit {
            return Err(bad());
        }
        last_unit = order;
        let part = decimal_nanos(number)
            .and_then(|n| n.checked_mul(seconds))
            .ok_or_else(bad)?;
        nanos = nanos.checked_add(part).ok_or_else(bad)?;
    }
    if last_unit == 0 {
        return Err(bad());
    }
    let secs = u64::try_from(nanos / NANOS_PER_SEC).map_err(|_| bad())?;
    let subsec = u32::try_from(nanos % NANOS_PER_SEC).map_err(|_| bad())?;
    Ok(Duration::new(secs, subsec))
}

/// `12.345` (or `12,345`) as nanoseconds; digits past nanoseconds are dropped.
fn decimal_nanos(number: &str) -> Option<u128> {
    let (whole, fraction) = match number.split_once(['.', ',']) {
        Some((whole, fraction)) if is_digits(fraction) => (whole, fraction),
        Some(_) => return None,
        None => (number, ""),
    };
    if !is_digits(whole) {
        return None;
    }
    let mut fraction_nanos = 0;
    let mut scale = NANOS_PER_SEC;
    for digit in fraction.bytes().take(9) {
        scale /= 10;
        fraction_nanos += u128::from(digit - b'0') * scale;
    }
    whole
        .parse::<u128>()
        .ok()?
        .checked_mul(NANOS_PER_SEC)?
        .checked_add(fraction_nanos)
}

fn duration_attribute(node: Node<'_, '_>, name: &str) -> Result<Option<Duration>, DownloadError> {
    node.attribute(name).map(parse_duration).transpose()
}

/// `"100-199"`: first and last byte, both included.
fn parse_range(text: &str) -> Result<ByteSpan, DownloadError> {
    let bad = || invalid("a byte range is not valid");
    let (first, last) = text.trim().split_once('-').ok_or_else(bad)?;
    let (first, last) = (
        unsigned(first).ok_or_else(bad)?,
        unsigned(last).ok_or_else(bad)?,
    );
    let length = last
        .checked_sub(first)
        .and_then(|n| n.checked_add(1))
        .ok_or_else(bad)?;
    Ok(ByteSpan {
        offset: first,
        length,
    })
}

fn unsigned(text: &str) -> Option<u64> {
    let text = text.trim();
    if is_digits(text) {
        text.parse().ok()
    } else {
        None
    }
}

fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

fn parse_number<T: FromStr>(name: &str, value: Option<&str>) -> Result<Option<T>, DownloadError> {
    value
        .map(|v| {
            v.trim()
                .parse()
                .map_err(|_| invalid(format!("`{name}` is not a valid number")))
        })
        .transpose()
}

fn text_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

/// Length of the period in `timescale` units, rounded up so a partial last
/// segment still counts.
fn period_ticks(len: Duration, timescale: u64) -> Result<u64, DownloadError> {
    let ticks = len
        .as_nanos()
        .checked_mul(u128::from(timescale))
        .ok_or_else(overflow)?
        .div_ceil(NANOS_PER_SEC);
    u64::try_from(ticks).map_err(|_| overflow())
}

/// `ticks` at `timescale` per second, in rounded milliseconds.
fn ticks_to_ms(ticks: u64, timescale: u64) -> u64 {
    let timescale = u128::from(timescale);
    let ms = (u128::from(ticks) * 1000 + timescale / 2)
        .checked_div(timescale)
        .unwrap_or(0);
    u64::try_from(ms).unwrap_or(u64::MAX)
}

fn millis(duration: Duration) -> u64 {
    u64::try_from((duration.as_nanos() + 500_000) / 1_000_000).unwrap_or(u64::MAX)
}

fn is_named(node: Node<'_, '_>, name: &str) -> bool {
    node.is_element() && node.tag_name().name() == name
}

fn elements<'a, 'i>(node: Node<'a, 'i>, name: &str) -> impl Iterator<Item = Node<'a, 'i>> {
    node.children().filter(move |n| is_named(*n, name))
}

fn child<'a, 'i>(node: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    elements(node, name).next()
}

fn invalid(message: impl Into<String>) -> DownloadError {
    DownloadError::InvalidManifest(message.into())
}

fn overflow() -> DownloadError {
    invalid("a number in the manifest is too large")
}

fn too_many_segments() -> DownloadError {
    DownloadError::UnsupportedMedia("the manifest has too many segments".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MPD_URL: &str = "https://cdn.example/vod/movie/manifest.mpd";

    fn manifest(xml: &str) -> Result<Manifest, DownloadError> {
        parse(xml, &Url::parse(MPD_URL).unwrap())
    }

    fn urls(segments: &[Segment]) -> Vec<&str> {
        segments.iter().map(|s| s.url.as_str()).collect()
    }

    fn rep<'m>(manifest: &'m Manifest, id: &str) -> &'m Representation {
        manifest
            .representations
            .iter()
            .find(|r| r.id == id)
            .unwrap()
    }

    fn assert_invalid(result: Result<Manifest, DownloadError>) {
        assert!(
            matches!(result, Err(DownloadError::InvalidManifest(_))),
            "{result:?}"
        );
    }

    #[test]
    fn template_with_number_width_and_duration() {
        let m = manifest(
            r#"<?xml version="1.0" encoding="UTF-8"?>
            <MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static"
                 mediaPresentationDuration="PT10.5S">
              <Period>
                <AdaptationSet mimeType="video/mp4">
                  <SegmentTemplate timescale="1000" duration="2000"
                    initialization="$RepresentationID$/init.mp4"
                    media="$RepresentationID$/seg-$Number%05d$.m4s"/>
                  <Representation id="v720" bandwidth="2000000" width="1280"
                    height="720" codecs="avc1.64001f"/>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        assert_eq!(m.duration_ms, Some(10_500));
        let v = rep(&m, "v720");
        assert_eq!(v.kind, ContentKind::Video);
        assert_eq!(
            (v.width, v.height, v.bandwidth),
            (Some(1280), Some(720), Some(2_000_000))
        );
        // 10.5 s of 2 s segments: five full ones and a short sixth.
        assert_eq!(v.segments.len(), 6);
        let all = urls(&v.segments);
        assert_eq!(all[0], "https://cdn.example/vod/movie/v720/seg-00001.m4s");
        assert_eq!(all[5], "https://cdn.example/vod/movie/v720/seg-00006.m4s");
        assert!(
            v.segments
                .iter()
                .all(|s| s.duration_ms == 2000 && s.range.is_none())
        );
        let init = v.init.as_ref().unwrap();
        assert_eq!(
            init.url.as_str(),
            "https://cdn.example/vod/movie/v720/init.mp4"
        );
        assert_eq!(init.range, None);
    }

    #[test]
    fn timeline_with_implicit_times_and_repeats() {
        let m = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT20S">
              <Period>
                <AdaptationSet mimeType="audio/mp4" lang="en">
                  <Representation id="a" bandwidth="128000" codecs="mp4a.40.2">
                    <SegmentTemplate timescale="48000" startNumber="10"
                        initialization="a/init.m4s" media="a/$Time$-$Number$.m4s">
                      <SegmentTimeline>
                        <S t="0" d="96000" r="1"/>
                        <S d="48000"/>
                        <S t="288000" d="96000" r="-1"/>
                      </SegmentTimeline>
                    </SegmentTemplate>
                  </Representation>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        let a = rep(&m, "a");
        assert_eq!(a.kind, ContentKind::Audio);
        let all = urls(&a.segments);
        // Two from r="1", one with an implicit t, then r="-1" fills the
        // 20 s (960000 ticks) from 288000 in steps of 96000: seven more.
        assert_eq!(all.len(), 10);
        assert_eq!(all[0], "https://cdn.example/vod/movie/a/0-10.m4s");
        assert_eq!(all[1], "https://cdn.example/vod/movie/a/96000-11.m4s");
        assert_eq!(all[2], "https://cdn.example/vod/movie/a/192000-12.m4s");
        assert_eq!(all[3], "https://cdn.example/vod/movie/a/288000-13.m4s");
        assert_eq!(all[9], "https://cdn.example/vod/movie/a/864000-19.m4s");
        let durations: Vec<u64> = a.segments.iter().map(|s| s.duration_ms).collect();
        assert_eq!(&durations[..4], &[2000, 2000, 1000, 2000]);
        assert_eq!(
            a.init.as_ref().unwrap().url.as_str(),
            "https://cdn.example/vod/movie/a/init.m4s"
        );
    }

    #[test]
    fn timeline_repeat_until_next_entry_or_unknown_end() {
        let m = manifest(
            r#"<MPD type="static">
              <Period>
                <AdaptationSet mimeType="video/mp4">
                  <SegmentTemplate media="$Time$.m4s">
                    <SegmentTimeline>
                      <S t="100" d="10" r="-1"/>
                      <S t="130" d="5"/>
                    </SegmentTimeline>
                  </SegmentTemplate>
                  <Representation id="v"/>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        let names: Vec<_> = rep(&m, "v")
            .segments
            .iter()
            .map(|s| s.url.path().rsplit('/').next().unwrap().to_string())
            .collect();
        assert_eq!(names, ["100.m4s", "110.m4s", "120.m4s", "130.m4s"]);

        // The last entry repeats to the end of the period, which is unknown.
        assert_invalid(manifest(
            r#"<MPD type="static"><Period><AdaptationSet mimeType="video/mp4">
              <SegmentTemplate media="$Time$.m4s">
                <SegmentTimeline><S d="10" r="-1"/></SegmentTimeline>
              </SegmentTemplate>
              <Representation id="v"/>
            </AdaptationSet></Period></MPD>"#,
        ));
    }

    #[test]
    fn representations_inherit_from_adaptation_sets() {
        let m = manifest(
            r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static"
                    mediaPresentationDuration="PT4S">
              <Period id="1">
                <AdaptationSet mimeType="video/mp4" codecs="avc1.4d401f" frameRate="25"
                    width="1920" height="1080">
                  <SegmentTemplate timescale="1" duration="2" startNumber="0"
                    media="$RepresentationID$_$Number$.m4s"
                    initialization="$RepresentationID$_init.m4s"/>
                  <Representation id="hd" bandwidth="5000000"/>
                  <Representation id="sd" bandwidth="1000000" width="640" height="360"
                      codecs="avc1.42c01e" frameRate="30000/1001">
                    <SegmentTemplate media="small/$Number$.m4s"/>
                  </Representation>
                </AdaptationSet>
                <AdaptationSet mimeType="audio/mp4" lang="en" codecs="mp4a.40.2">
                  <Role schemeIdUri="urn:mpeg:dash:role:2011" value="main"/>
                  <Label>English</Label>
                  <SegmentTemplate timescale="1" duration="2"
                    media="audio/$Bandwidth$/$Number$.m4s"
                    initialization="audio/$Bandwidth$/init.m4s"/>
                  <Representation bandwidth="96000"/>
                </AdaptationSet>
                <AdaptationSet contentType="audio" lang="de">
                  <Role schemeIdUri="urn:mpeg:dash:role:2011" value="alternate"/>
                  <Label>German</Label>
                  <Representation id="de" bandwidth="64000" mimeType="audio/mp4"
                      codecs="opus">
                    <Label>Deutsch</Label>
                    <BaseURL>de.mp4</BaseURL>
                  </Representation>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        let ids: Vec<_> = m.representations.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["hd", "sd", "1-0", "de"]);

        let hd = rep(&m, "hd");
        assert_eq!(hd.kind, ContentKind::Video);
        assert_eq!(hd.mime_type.as_deref(), Some("video/mp4"));
        assert_eq!(hd.codecs.as_deref(), Some("avc1.4d401f"));
        assert_eq!((hd.width, hd.height), (Some(1920), Some(1080)));
        assert_eq!(hd.frame_rate.as_deref(), Some("25"));
        assert_eq!(
            (hd.language.as_deref(), hd.label.as_deref(), hd.main),
            (None, None, false)
        );
        assert_eq!(
            urls(&hd.segments),
            [
                "https://cdn.example/vod/movie/hd_0.m4s",
                "https://cdn.example/vod/movie/hd_1.m4s"
            ]
        );

        // Its own template replaces `media` only; the rest comes from above.
        let sd = rep(&m, "sd");
        assert_eq!((sd.width, sd.height), (Some(640), Some(360)));
        assert_eq!(sd.codecs.as_deref(), Some("avc1.42c01e"));
        assert_eq!(sd.frame_rate.as_deref(), Some("30000/1001"));
        assert_eq!(
            urls(&sd.segments),
            [
                "https://cdn.example/vod/movie/small/0.m4s",
                "https://cdn.example/vod/movie/small/1.m4s"
            ]
        );
        assert_eq!(
            sd.init.as_ref().unwrap().url.as_str(),
            "https://cdn.example/vod/movie/sd_init.m4s"
        );

        let en = rep(&m, "1-0");
        assert_eq!(en.kind, ContentKind::Audio);
        assert_eq!(en.language.as_deref(), Some("en"));
        assert_eq!(en.label.as_deref(), Some("English"));
        assert!(en.main);
        assert_eq!(en.codecs.as_deref(), Some("mp4a.40.2"));
        assert_eq!(
            en.segments[0].url.as_str(),
            "https://cdn.example/vod/movie/audio/96000/1.m4s"
        );
        assert_eq!(
            en.init.as_ref().unwrap().url.as_str(),
            "https://cdn.example/vod/movie/audio/96000/init.m4s"
        );

        let de = rep(&m, "de");
        assert_eq!(de.kind, ContentKind::Audio);
        assert_eq!(de.language.as_deref(), Some("de"));
        assert_eq!(de.label.as_deref(), Some("Deutsch"));
        assert!(!de.main);
        assert_eq!(urls(&de.segments), ["https://cdn.example/vod/movie/de.mp4"]);
    }

    #[test]
    fn segment_list_with_ranges() {
        let m = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT30S">
              <Period>
                <AdaptationSet mimeType="video/mp4">
                  <Representation id="v" bandwidth="800000">
                    <BaseURL>video.mp4</BaseURL>
                    <SegmentList timescale="1000" duration="10000">
                      <Initialization range="0-861"/>
                      <SegmentURL mediaRange="862-50000"/>
                      <SegmentURL mediaRange="50001-99999"/>
                      <SegmentURL media="tail.mp4" mediaRange="0-99"/>
                    </SegmentList>
                  </Representation>
                  <Representation id="w" bandwidth="400000">
                    <SegmentList duration="4">
                      <Initialization sourceURL="w/init.mp4"/>
                      <SegmentURL media="w/1.mp4"/>
                      <SegmentURL media="w/2.mp4"/>
                    </SegmentList>
                  </Representation>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        let v = rep(&m, "v");
        let init = v.init.as_ref().unwrap();
        assert_eq!(init.url.as_str(), "https://cdn.example/vod/movie/video.mp4");
        assert_eq!(
            init.range,
            Some(ByteSpan {
                offset: 0,
                length: 862
            })
        );
        let ranges: Vec<_> = v.segments.iter().map(|s| s.range).collect();
        assert_eq!(
            ranges,
            [
                Some(ByteSpan {
                    offset: 862,
                    length: 49_139
                }),
                Some(ByteSpan {
                    offset: 50_001,
                    length: 49_999
                }),
                Some(ByteSpan {
                    offset: 0,
                    length: 100
                }),
            ]
        );
        assert_eq!(
            urls(&v.segments),
            [
                "https://cdn.example/vod/movie/video.mp4",
                "https://cdn.example/vod/movie/video.mp4",
                "https://cdn.example/vod/movie/tail.mp4"
            ]
        );
        assert!(v.segments.iter().all(|s| s.duration_ms == 10_000));

        let w = rep(&m, "w");
        let init = w.init.as_ref().unwrap();
        assert_eq!(
            init.url.as_str(),
            "https://cdn.example/vod/movie/w/init.mp4"
        );
        assert_eq!(init.range, None);
        assert_eq!(w.segments.len(), 2);
        assert_eq!(w.segments[1].duration_ms, 4000);
    }

    #[test]
    fn segment_base_is_one_file() {
        let m = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT1M">
              <Period>
                <AdaptationSet mimeType="audio/mp4">
                  <Representation id="a" bandwidth="128000">
                    <BaseURL>audio_128k.mp4</BaseURL>
                    <SegmentBase indexRange="800-1500">
                      <Initialization range="0-799"/>
                    </SegmentBase>
                  </Representation>
                </AdaptationSet>
                <AdaptationSet mimeType="text/vtt" lang="fr">
                  <Representation id="sub">
                    <BaseURL>subs/fr.vtt</BaseURL>
                  </Representation>
                </AdaptationSet>
              </Period>
            </MPD>"#,
        )
        .unwrap();
        let a = rep(&m, "a");
        assert_eq!(a.init, None);
        assert_eq!(a.segments.len(), 1);
        assert_eq!(
            a.segments[0].url.as_str(),
            "https://cdn.example/vod/movie/audio_128k.mp4"
        );
        assert_eq!(a.segments[0].range, None);
        assert_eq!(a.segments[0].duration_ms, 60_000);

        let sub = rep(&m, "sub");
        assert_eq!(sub.kind, ContentKind::Text);
        assert_eq!(sub.init, None);
        assert_eq!(
            urls(&sub.segments),
            ["https://cdn.example/vod/movie/subs/fr.vtt"]
        );
    }

    #[test]
    fn base_urls_chain_through_levels() {
        let m = parse(
            r#"<MPD type="static" mediaPresentationDuration="PT2S">
              <BaseURL>media/</BaseURL>
              <Period>
                <BaseURL>period1/</BaseURL>
                <AdaptationSet mimeType="video/mp4">
                  <BaseURL>video/</BaseURL>
                  <SegmentTemplate media="$Number$.m4s" duration="2"/>
                  <Representation id="v1"><BaseURL>hd/</BaseURL></Representation>
                  <Representation id="v2">
                    <BaseURL>https://cdn2.example/other/</BaseURL>
                  </Representation>
                  <Representation id="v3"><BaseURL>/root/</BaseURL></Representation>
                  <Representation id="v4"/>
                </AdaptationSet>
              </Period>
            </MPD>"#,
            &Url::parse("https://origin.example/content/manifest.mpd?token=abc").unwrap(),
        )
        .unwrap();
        let first = |id: &str| rep(&m, id).segments[0].url.to_string();
        assert_eq!(
            first("v1"),
            "https://origin.example/content/media/period1/video/hd/1.m4s"
        );
        assert_eq!(first("v2"), "https://cdn2.example/other/1.m4s");
        assert_eq!(first("v3"), "https://origin.example/root/1.m4s");
        assert_eq!(
            first("v4"),
            "https://origin.example/content/media/period1/video/1.m4s"
        );

        // An absolute BaseURL at the top replaces the manifest's location.
        let m = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT2S">
              <BaseURL>http://other.example/a/</BaseURL>
              <Period><AdaptationSet mimeType="video/mp4">
                <Representation id="v"><BaseURL>b.mp4</BaseURL></Representation>
              </AdaptationSet></Period>
            </MPD>"#,
        )
        .unwrap();
        assert_eq!(
            rep(&m, "v").segments[0].url.as_str(),
            "http://other.example/a/b.mp4"
        );
    }

    #[test]
    fn content_protection_anywhere_is_refused() {
        let places = [
            r#"<MPD type="static" mediaPresentationDuration="PT2S"><Period>
              <AdaptationSet mimeType="video/mp4">
                <ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011" value="cenc"/>
                <Representation id="v"><BaseURL>v.mp4</BaseURL></Representation>
              </AdaptationSet></Period></MPD>"#,
            r#"<MPD type="static" mediaPresentationDuration="PT2S"><Period>
              <AdaptationSet mimeType="video/mp4">
                <Representation id="v"><BaseURL>v.mp4</BaseURL></Representation>
              </AdaptationSet>
              <AdaptationSet mimeType="audio/mp4">
                <Representation id="a">
                  <ContentProtection schemeIdUri="urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed"/>
                  <BaseURL>a.mp4</BaseURL>
                </Representation>
              </AdaptationSet></Period></MPD>"#,
            r#"<mpd:MPD xmlns:mpd="urn:mpeg:dash:schema:mpd:2011" type="static">
              <mpd:Period><mpd:AdaptationSet>
                <mpd:ContentProtection schemeIdUri="urn:mpeg:dash:mp4protection:2011"/>
              </mpd:AdaptationSet></mpd:Period></mpd:MPD>"#,
        ];
        for xml in places {
            assert!(
                matches!(manifest(xml), Err(DownloadError::Protected)),
                "{xml}"
            );
        }
    }

    #[test]
    fn live_and_multi_period_manifests_are_refused() {
        let live = manifest(
            r#"<MPD type="dynamic" availabilityStartTime="2026-01-01T00:00:00Z"><Period>
              <AdaptationSet mimeType="video/mp4">
                <SegmentTemplate media="$Number$.m4s" duration="2"/>
                <Representation id="v"/>
              </AdaptationSet></Period></MPD>"#,
        );
        assert!(matches!(live, Err(DownloadError::LiveStream)), "{live:?}");

        let periods = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT4S">
              <Period duration="PT2S"><AdaptationSet mimeType="video/mp4">
                <Representation id="v"><BaseURL>a.mp4</BaseURL></Representation>
              </AdaptationSet></Period>
              <Period duration="PT2S"><AdaptationSet mimeType="video/mp4">
                <Representation id="v"><BaseURL>b.mp4</BaseURL></Representation>
              </AdaptationSet></Period>
            </MPD>"#,
        );
        assert!(
            matches!(periods, Err(DownloadError::UnsupportedMedia(_))),
            "{periods:?}"
        );
    }

    #[test]
    fn broken_documents_are_invalid() {
        assert_invalid(manifest("<html><body>not a manifest</body></html>"));
        assert_invalid(manifest("<MPD type=\"static\"><Period>"));
        assert_invalid(manifest(""));
        assert_invalid(manifest(
            r#"<?xml version="1.0"?>
            <!DOCTYPE MPD [ <!ENTITY a "aaaaaaaaaa"> <!ENTITY b "&a;&a;&a;&a;"> ]>
            <MPD type="static"><Period><AdaptationSet mimeType="video/mp4">
              <Representation id="&b;"><BaseURL>v.mp4</BaseURL></Representation>
            </AdaptationSet></Period></MPD>"#,
        ));
        // No Period, or nothing in it.
        assert_invalid(manifest(r#"<MPD type="static"/>"#));
        assert_invalid(manifest(r#"<MPD type="static"><Period/></MPD>"#));
    }

    #[test]
    fn unsafe_values_are_refused() {
        let with = |template: &str| {
            manifest(&format!(
                r#"<MPD type="static" mediaPresentationDuration="PT100000.5S"><Period>
                  <AdaptationSet mimeType="video/mp4">{template}
                    <Representation id="v"/>
                  </AdaptationSet></Period></MPD>"#
            ))
        };
        assert_invalid(with(
            r#"<SegmentTemplate media="$Number$.m4s" duration="2" timescale="0"/>"#,
        ));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Number.m4s" duration="2"/>"#,
        ));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Name$.m4s" duration="2"/>"#,
        ));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Time$.m4s" duration="2"/>"#,
        ));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Number$.m4s" duration="0"/>"#,
        ));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Number$.m4s" duration="x"/>"#,
        ));
        assert_invalid(with(r#"<BaseURL>file:///etc/</BaseURL>"#));
        assert_invalid(with(
            r#"<SegmentTemplate media="$Number$.m4s" startNumber="18446744073709551615">
                 <SegmentTimeline><S d="1" r="1"/></SegmentTimeline>
               </SegmentTemplate>"#,
        ));
        // 100000.5 s of one-second segments is one too many.
        let many = with(r#"<SegmentTemplate media="$Number$.m4s" duration="1"/>"#);
        assert!(
            matches!(many, Err(DownloadError::UnsupportedMedia(_))),
            "{many:?}"
        );
        let repeats = with(
            r#"<SegmentTemplate media="$Time$.m4s"><SegmentTimeline>
                 <S d="1" r="9223372036854775807"/>
               </SegmentTimeline></SegmentTemplate>"#,
        );
        assert!(
            matches!(repeats, Err(DownloadError::UnsupportedMedia(_))),
            "{repeats:?}"
        );
        assert_invalid(with(
            r#"<SegmentTemplate media="$Time$.m4s"><SegmentTimeline>
                 <S t="18446744073709551615" d="1"/><S d="1"/>
               </SegmentTimeline></SegmentTemplate>"#,
        ));
    }

    #[test]
    fn end_number_bounds_the_template() {
        let m = manifest(
            r#"<MPD type="static"><Period>
              <AdaptationSet mimeType="video/mp4">
                <SegmentTemplate media="seg-$Number$.m4s" timescale="10" duration="40"
                    startNumber="5" endNumber="8"/>
                <Representation id="v" bandwidth="1"/>
              </AdaptationSet></Period></MPD>"#,
        )
        .unwrap();
        assert_eq!(m.duration_ms, None);
        let v = rep(&m, "v");
        let names: Vec<_> = v
            .segments
            .iter()
            .map(|s| s.url.path().rsplit('/').next().unwrap().to_string())
            .collect();
        assert_eq!(names, ["seg-5.m4s", "seg-6.m4s", "seg-7.m4s", "seg-8.m4s"]);
        assert!(v.segments.iter().all(|s| s.duration_ms == 4000));
        assert_eq!(v.init, None);

        assert_invalid(manifest(
            r#"<MPD type="static"><Period><AdaptationSet mimeType="video/mp4">
                <SegmentTemplate media="$Number$.m4s" duration="4" startNumber="5" endNumber="4"/>
                <Representation id="v"/>
              </AdaptationSet></Period></MPD>"#,
        ));
    }

    #[test]
    fn kinds_from_types_and_codecs() {
        let m = manifest(
            r#"<MPD type="static" mediaPresentationDuration="PT2S"><Period>
              <AdaptationSet mimeType="application/mp4" codecs="stpp.ttml.im1t">
                <Representation id="ttml"><BaseURL>t.mp4</BaseURL></Representation>
              </AdaptationSet>
              <AdaptationSet>
                <ContentComponent contentType="audio"/>
                <ContentComponent contentType="video"/>
                <Representation id="muxed"><BaseURL>m.mp4</BaseURL></Representation>
              </AdaptationSet>
              <AdaptationSet>
                <Representation id="codecs" codecs="ec-3"><BaseURL>e.mp4</BaseURL></Representation>
                <Representation id="unknown"><BaseURL>u.bin</BaseURL></Representation>
                <Representation id="ttml-xml" mimeType="application/ttml+xml">
                  <BaseURL>s.ttml</BaseURL>
                </Representation>
              </AdaptationSet>
            </Period></MPD>"#,
        )
        .unwrap();
        let kind = |id: &str| rep(&m, id).kind;
        assert_eq!(kind("ttml"), ContentKind::Text);
        assert_eq!(kind("muxed"), ContentKind::Video);
        assert_eq!(kind("codecs"), ContentKind::Audio);
        assert_eq!(kind("unknown"), ContentKind::Other);
        assert_eq!(kind("ttml-xml"), ContentKind::Text);
        assert_eq!(codecs_kind("avc1.64001f, mp4a.40.2"), ContentKind::Video);
        assert_eq!(codecs_kind("wvtt"), ContentKind::Text);
    }

    #[test]
    fn iso_durations() {
        let secs = |s: &str| parse_duration(s).map(|d| d.as_secs_f64()).ok();
        assert_eq!(secs("PT1H2M3.5S"), Some(3723.5));
        assert_eq!(secs("PT0.5S"), Some(0.5));
        assert_eq!(secs("P1DT1S"), Some(86_401.0));
        assert_eq!(secs("PT90S"), Some(90.0));
        assert_eq!(secs("PT1M"), Some(60.0));
        assert_eq!(secs("P2D"), Some(172_800.0));
        assert_eq!(secs(" PT0,25S "), Some(0.25));
        assert_eq!(
            parse_duration("PT0.1234567891S").unwrap(),
            Duration::from_nanos(123_456_789)
        );
        assert_eq!(millis(parse_duration("PT634.5665S").unwrap()), 634_567);
        for bad in [
            "",
            "P",
            "PT",
            "P1DT",
            "1S",
            "PT1",
            "PT-1S",
            "PT1.S",
            "PT.5S",
            "PT1.5.5S",
            "PT1S2M",
            "PT1H1H",
            "P1H",
            "PT1D",
            "P1Y",
            "P1M",
            "P1Y2M3DT4S",
            "P1W",
            "PT1X",
            "PT99999999999999999999999999999999999999999S",
        ] {
            assert!(
                matches!(parse_duration(bad), Err(DownloadError::InvalidManifest(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn template_identifiers() {
        let vars = TemplateVars {
            representation_id: "video=1",
            bandwidth: Some(2_500_000),
            number: Some(42),
            time: Some(90_000),
        };
        let fill = |pattern: &str| fill_template(pattern, &vars).ok();
        assert_eq!(
            fill("$RepresentationID$/$Number%05d$.m4s").as_deref(),
            Some("video=1/00042.m4s")
        );
        assert_eq!(
            fill("b$Bandwidth$-t$Time$").as_deref(),
            Some("b2500000-t90000")
        );
        assert_eq!(
            fill("$Bandwidth%09d$_$Time%3d$").as_deref(),
            Some("002500000_90000")
        );
        assert_eq!(fill("cost$$5-$Number%d$").as_deref(), Some("cost$5-42"));
        assert_eq!(fill("plain.mp4").as_deref(), Some("plain.mp4"));
        assert_eq!(fill("").as_deref(), Some(""));
        for bad in [
            "$Number",
            "a$",
            "$SubNumber$",
            "$number$",
            "$RepresentationID%05d$",
            "$Number%05x$",
            "$Number%-5d$",
            "$Number%099d$",
            "$%05d$",
        ] {
            assert_eq!(fill(bad), None, "{bad:?}");
        }
        let init = TemplateVars {
            representation_id: "a",
            bandwidth: None,
            number: None,
            time: None,
        };
        assert!(fill_template("$Number$", &init).is_err());
        assert!(fill_template("$Bandwidth$", &init).is_err());
        assert_eq!(fill_template("$RepresentationID$", &init).unwrap(), "a");
    }

    #[test]
    fn byte_ranges() {
        assert_eq!(
            parse_range("0-0").unwrap(),
            ByteSpan {
                offset: 0,
                length: 1
            }
        );
        assert_eq!(
            parse_range(" 100-199 ").unwrap(),
            ByteSpan {
                offset: 100,
                length: 100
            }
        );
        for bad in [
            "",
            "5",
            "5-",
            "-5",
            "9-5",
            "a-b",
            "+1-2",
            "0-18446744073709551615",
        ] {
            assert!(parse_range(bad).is_err(), "{bad:?}");
        }
    }
}
