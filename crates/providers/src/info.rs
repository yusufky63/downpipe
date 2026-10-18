//! What a page offers, read from `yt-dlp -J`.
//!
//! yt-dlp prints everything it knows about a video as one JSON object. Only
//! the fields below are read, each checked for type and size, and format ids
//! are kept only when they are plain tokens, because they go back to yt-dlp
//! in a `-f` format selector. The formats are condensed into the same
//! quality list the stream chooser shows: one entry per resolution and
//! frame rate, the separate audio tracks, and "Audio only".

use std::collections::HashMap;
use std::path::PathBuf;

use downpipe_http::DownloadError;
use downpipe_http::media::{AudioOption, VideoOption};
use serde::{Deserialize, Serialize};

use crate::args;
use crate::errors;
use crate::process;
use crate::tools::Tools;

/// `-J` output larger than this is refused (a long video with every caption
/// language is a few megabytes).
const MAX_JSON: usize = 64 * 1024 * 1024;
const MAX_TITLE: usize = 200;

/// A video page: its title and the qualities to choose from, best first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageInfo {
    /// The video's id on the site.
    pub id: Option<String>,
    pub title: Option<String>,
    /// The channel or uploader, for the file name template.
    pub channel: Option<String>,
    /// `YYYYMMDD`, for the file name template.
    pub upload_date: Option<String>,
    /// The site as yt-dlp names its extractor ("Youtube").
    pub extractor: Option<String>,
    pub duration_ms: Option<u64>,
    /// Video qualities first, then "Audio only" entries.
    pub videos: Vec<VideoOption>,
    /// Separate sound tracks that go with the video-only qualities.
    pub audios: Vec<AudioOption>,
    /// Subtitles the uploader added, then the site's automatic captions.
    pub subtitles: Vec<SubtitleOption>,
}

/// A subtitle language a page offers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SubtitleOption {
    /// The site's code (`en`, `pt-BR`, `en-orig`), checked with
    /// [`args::valid_sub_lang`] because it goes back to yt-dlp.
    pub language: String,
    /// The site's name for it ("English (auto-generated)"), if it gave one.
    pub name: Option<String>,
    /// Captions the site generated from the speech (and their machine
    /// translations), rather than subtitles the uploader added.
    pub auto: bool,
}

/// Automatic captions kept: YouTube offers a machine translation into
/// about 150 languages for every video.
const MAX_AUTO_CAPTIONS: usize = 200;

#[derive(Debug, Default, Deserialize)]
struct RawInfo {
    #[serde(rename = "_type", default)]
    kind: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    uploader: Option<String>,
    #[serde(default)]
    upload_date: Option<String>,
    #[serde(default)]
    subtitles: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    automatic_captions: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    extractor_key: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    is_live: Option<bool>,
    #[serde(default)]
    live_status: Option<String>,
    #[serde(rename = "_has_drm", default)]
    has_drm: Option<bool>,
    #[serde(default)]
    formats: Vec<RawFormat>,
}

#[derive(Debug, Default, Deserialize)]
struct RawFormat {
    #[serde(default)]
    format_id: Option<String>,
    #[serde(default)]
    vcodec: Option<String>,
    #[serde(default)]
    acodec: Option<String>,
    #[serde(default)]
    width: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
    #[serde(default)]
    fps: Option<f64>,
    /// Kilobits per second, all streams.
    #[serde(default)]
    tbr: Option<f64>,
    #[serde(default)]
    abr: Option<f64>,
    #[serde(default)]
    filesize: Option<f64>,
    #[serde(default)]
    filesize_approx: Option<f64>,
    #[serde(default)]
    protocol: Option<String>,
    #[serde(default)]
    format_note: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    language_preference: Option<f64>,
    /// `true`, `false` or `"maybe"`.
    #[serde(default)]
    has_drm: Option<serde_json::Value>,
}

/// Ask yt-dlp about a page. Downloads nothing but the page and its player
/// data.
pub async fn inspect(
    tools: &Tools,
    url: &str,
    proxy: Option<&str>,
) -> Result<PageInfo, DownloadError> {
    let yt_dlp = tools.yt_dlp.as_ref().ok_or_else(errors::no_yt_dlp)?;
    let mut command = process::command(yt_dlp);
    command
        .args(args::inspect(url, tools, proxy))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    let child = command
        .spawn()
        .map_err(|e| errors::spawn_failed(yt_dlp, &e))?;
    let output = child.wait_with_output().await.map_err(DownloadError::Io)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let lines: Vec<String> = stderr.lines().map(str::to_string).collect();
        return Err(errors::classify(
            &lines,
            output.status.code(),
            tools.ffmpeg.is_some(),
        ));
    }
    parse(&output.stdout)
}

/// Turn `-J` output into a [`PageInfo`], refusing live streams, playlists
/// and protected videos.
pub fn parse(json: &[u8]) -> Result<PageInfo, DownloadError> {
    if json.len() > MAX_JSON {
        return Err(DownloadError::Extractor(
            "yt-dlp described this page with more data than Downpipe reads".into(),
        ));
    }
    let raw: RawInfo = serde_json::from_slice(json)
        .map_err(|e| DownloadError::Extractor(format!("yt-dlp's answer is not readable: {e}")))?;
    if raw.kind.as_deref().is_some_and(|k| k != "video") {
        return Err(DownloadError::UnsupportedMedia(
            "this is a playlist or channel; open a single video".into(),
        ));
    }
    if raw.is_live == Some(true)
        || matches!(raw.live_status.as_deref(), Some("is_live" | "is_upcoming"))
    {
        return Err(DownloadError::LiveStream);
    }
    let usable: Vec<&RawFormat> = raw
        .formats
        .iter()
        .filter(|f| f.format_id.as_deref().is_some_and(args::valid_format_id))
        .filter(|f| {
            f.has_drm
                .as_ref()
                .is_none_or(|d| d == &serde_json::Value::Bool(false))
        })
        .collect();
    if usable.is_empty() {
        let protected = raw.has_drm == Some(true)
            || raw.formats.iter().any(|f| {
                f.has_drm
                    .as_ref()
                    .is_some_and(|d| d != &serde_json::Value::Bool(false))
            });
        return Err(if protected {
            DownloadError::Protected
        } else {
            DownloadError::Extractor("yt-dlp found no video on this page".into())
        });
    }
    let duration_ms = raw
        .duration
        .filter(|d| d.is_finite() && *d > 0.0)
        .map(|d| (d * 1000.0) as u64);
    let (audios, audio_only) = audio_choices(&usable, duration_ms);
    let mut videos = video_choices(&usable, duration_ms);
    videos.extend(audio_only);
    let mut subtitles = subtitle_choices(raw.subtitles.as_ref(), false);
    subtitles.extend(subtitle_choices(raw.automatic_captions.as_ref(), true));
    Ok(PageInfo {
        id: raw
            .id
            .as_deref()
            .filter(|id| args::valid_format_id(id))
            .map(str::to_string),
        title: raw.title.as_deref().and_then(clean_title),
        channel: raw
            .channel
            .as_deref()
            .or(raw.uploader.as_deref())
            .and_then(clean_title),
        upload_date: raw
            .upload_date
            .filter(|d| d.len() == 8 && d.bytes().all(|b| b.is_ascii_digit())),
        extractor: raw.extractor_key.as_deref().and_then(clean_title),
        duration_ms,
        videos,
        audios,
        subtitles,
    })
}

/// One entry per language of yt-dlp's `subtitles` or `automatic_captions`
/// map (language code to a list of formats). The original language of
/// automatic captions (`en-orig`) comes first; YouTube's live chat replay,
/// which yt-dlp lists as a subtitle, is left out.
fn subtitle_choices(
    map: Option<&HashMap<String, serde_json::Value>>,
    auto: bool,
) -> Vec<SubtitleOption> {
    let Some(map) = map else {
        return Vec::new();
    };
    let mut out: Vec<SubtitleOption> = map
        .iter()
        .filter(|(lang, formats)| {
            lang.as_str() != "live_chat"
                && args::valid_sub_lang(lang)
                && formats.as_array().is_some_and(|f| !f.is_empty())
        })
        .map(|(lang, formats)| SubtitleOption {
            language: lang.clone(),
            name: formats
                .as_array()
                .and_then(|f| f.iter().find_map(|f| f.get("name")?.as_str()))
                .and_then(clean_title)
                .map(|n| n.chars().take(80).collect()),
            auto,
        })
        .collect();
    out.sort_by(|a, b| {
        let orig = |s: &SubtitleOption| !s.language.ends_with("-orig");
        orig(a).cmp(&orig(b)).then(a.language.cmp(&b.language))
    });
    out.truncate(MAX_AUTO_CAPTIONS);
    out
}

/// A title without control characters, capped.
fn clean_title(raw: &str) -> Option<String> {
    let title: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_TITLE)
        .collect();
    let title = title.trim();
    (!title.is_empty()).then(|| title.to_string())
}

fn has(codec: Option<&str>) -> bool {
    codec.is_some_and(|c| !c.is_empty() && c != "none")
}

/// Formats that play everywhere first: H.264 and AAC join into MP4.
fn video_rank(codec: &str) -> u8 {
    let c = codec.to_ascii_lowercase();
    if c.starts_with("avc") || c.starts_with("h264") {
        0
    } else if c.starts_with("av01") {
        1
    } else if c.starts_with("vp09") || c.starts_with("vp9") {
        2
    } else {
        3
    }
}

fn audio_family(codec: &str) -> &'static str {
    let c = codec.to_ascii_lowercase();
    if c.starts_with("mp4a") || c.starts_with("aac") {
        "aac"
    } else if c.starts_with("opus") {
        "opus"
    } else {
        "other"
    }
}

/// Direct downloads before HLS copies of the same format.
fn protocol_rank(f: &RawFormat) -> u8 {
    match f.protocol.as_deref() {
        Some("https" | "http") | None => 0,
        _ => 1,
    }
}

fn size_of(f: &RawFormat, duration_ms: Option<u64>) -> Option<u64> {
    f.filesize
        .or(f.filesize_approx)
        .filter(|s| s.is_finite() && *s > 0.0)
        .map(|s| s as u64)
        .or_else(|| {
            let kbps = f.tbr.filter(|t| t.is_finite() && *t > 0.0)?;
            Some((kbps * 1000.0 / 8.0 * duration_ms? as f64 / 1000.0) as u64)
        })
}

fn bits(kbps: Option<f64>) -> Option<u64> {
    kbps.filter(|k| k.is_finite() && *k > 0.0)
        .map(|k| (k * 1000.0).round() as u64)
}

fn fps_text(fps: Option<f64>) -> Option<String> {
    let fps = fps.filter(|f| f.is_finite() && *f > 0.0)?;
    Some(if (fps - fps.round()).abs() < 0.01 {
        format!("{}", fps.round() as u64)
    } else {
        format!("{fps:.2}")
    })
}

fn dimension(v: Option<f64>) -> Option<u32> {
    v.filter(|v| v.is_finite() && *v > 0.0 && *v < 100_000.0)
        .map(|v| v as u32)
}

/// One entry per resolution and frame rate, best first, with or without
/// sound as separate entries; the best format of each by codec, then
/// protocol, then bit rate.
fn video_choices(formats: &[&RawFormat], duration_ms: Option<u64>) -> Vec<VideoOption> {
    let mut best: HashMap<(u32, u64, bool), &RawFormat> = HashMap::new();
    for f in formats.iter().copied() {
        let (Some(height), true) = (dimension(f.height), has(f.vcodec.as_deref())) else {
            continue;
        };
        let fps = f
            .fps
            .filter(|v| v.is_finite())
            .map_or(0, |v| v.round() as u64);
        let key = (height, fps, has(f.acodec.as_deref()));
        let better = |old: &RawFormat| {
            let rank = |f: &RawFormat| {
                (
                    video_rank(f.vcodec.as_deref().unwrap_or("")),
                    protocol_rank(f),
                )
            };
            rank(f) < rank(old)
                || (rank(f) == rank(old) && f.tbr.unwrap_or(0.0) > old.tbr.unwrap_or(0.0))
        };
        match best.get(&key) {
            Some(old) if !better(old) => {}
            _ => {
                best.insert(key, f);
            }
        }
    }
    let mut entries: Vec<((u32, u64, bool), &RawFormat)> = best.into_iter().collect();
    // Higher first; at the same resolution, video-only (which joins the best
    // sound) before formats that carry their own.
    entries.sort_by(|(a, _), (b, _)| b.0.cmp(&a.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
    entries
        .into_iter()
        .map(|((_, _, with_audio), f)| {
            let codecs = [f.vcodec.as_deref(), f.acodec.as_deref()]
                .into_iter()
                .filter(|c| has(*c))
                .flatten()
                .collect::<Vec<_>>()
                .join(", ");
            VideoOption {
                id: f.format_id.clone().unwrap_or_default(),
                width: dimension(f.width),
                height: dimension(f.height),
                bandwidth: bits(f.tbr),
                codecs: (!codecs.is_empty()).then_some(codecs),
                frame_rate: fps_text(f.fps),
                has_audio: with_audio,
                audio_group: None,
                audio_only: false,
                estimated_size: size_of(f, duration_ms),
            }
        })
        .collect()
}

/// The sound tracks (one per language and codec, without YouTube's
/// "dynamic range compressed" copies when the normal one exists), and the
/// "Audio only" entries of the main language.
fn audio_choices(
    formats: &[&RawFormat],
    duration_ms: Option<u64>,
) -> (Vec<AudioOption>, Vec<VideoOption>) {
    let audio: Vec<&RawFormat> = formats
        .iter()
        .copied()
        .filter(|f| has(f.acodec.as_deref()) && !has(f.vcodec.as_deref()))
        .collect();
    let is_drc = |f: &RawFormat| {
        f.format_id
            .as_deref()
            .is_some_and(|id| id.ends_with("-drc"))
            || f.format_note.as_deref().is_some_and(|n| n.contains("DRC"))
    };
    let any_plain = audio.iter().any(|f| !is_drc(f));
    let mut best: HashMap<(String, &'static str), &RawFormat> = HashMap::new();
    for f in audio.iter().copied().filter(|f| !any_plain || !is_drc(f)) {
        let key = (
            f.language.clone().unwrap_or_default(),
            audio_family(f.acodec.as_deref().unwrap_or("")),
        );
        let rate = |f: &RawFormat| f.abr.or(f.tbr).unwrap_or(0.0);
        let better = |old: &RawFormat| {
            protocol_rank(f) < protocol_rank(old)
                || (protocol_rank(f) == protocol_rank(old) && rate(f) > rate(old))
        };
        match best.get(&key) {
            Some(old) if !better(old) => {}
            _ => {
                best.insert(key, f);
            }
        }
    }
    // The original language (yt-dlp gives it the highest preference) first,
    // AAC before Opus because it joins H.264 into an MP4.
    let preference = |f: &RawFormat| f.language_preference.unwrap_or(0.0);
    let main_language = best
        .values()
        .max_by(|a, b| preference(a).total_cmp(&preference(b)))
        .and_then(|f| f.language.clone());
    let family_rank = |f: &RawFormat| match audio_family(f.acodec.as_deref().unwrap_or("")) {
        "aac" => 0,
        "opus" => 1,
        _ => 2,
    };
    let mut tracks: Vec<&RawFormat> = best.into_values().collect();
    tracks.sort_by(|a, b| {
        let main = |f: &RawFormat| f.language != main_language;
        main(a)
            .cmp(&main(b))
            .then(preference(b).total_cmp(&preference(a)))
            .then(a.language.cmp(&b.language))
            .then(family_rank(a).cmp(&family_rank(b)))
    });
    let audios: Vec<AudioOption> = tracks
        .iter()
        .enumerate()
        .map(|(i, f)| AudioOption {
            id: f.format_id.clone().unwrap_or_default(),
            name: None,
            language: f.language.clone(),
            bandwidth: bits(f.abr.or(f.tbr)),
            codecs: f.acodec.clone(),
            default: i == 0,
            group: None,
        })
        .collect();
    let audio_only = tracks
        .iter()
        .filter(|f| f.language == main_language)
        .map(|f| VideoOption {
            id: f.format_id.clone().unwrap_or_default(),
            width: None,
            height: None,
            bandwidth: bits(f.abr.or(f.tbr)),
            codecs: f.acodec.clone(),
            frame_rate: None,
            has_audio: true,
            audio_group: None,
            audio_only: true,
            estimated_size: size_of(f, duration_ms),
        })
        .collect();
    (audios, audio_only)
}

/// Where `-J` output for tests lives.
#[doc(hidden)]
pub fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn read(name: &str) -> Vec<u8> {
        std::fs::read(fixture_dir().join(name)).unwrap()
    }

    #[test]
    fn condenses_youtube_formats() {
        let info = parse(&read("youtube-info.json")).unwrap();
        assert_eq!(info.title.as_deref(), Some("Me at the zoo"));
        assert_eq!(info.id.as_deref(), Some("jNQXAC9IVRw"));
        assert_eq!(info.channel.as_deref(), Some("jawed"));
        assert_eq!(info.upload_date.as_deref(), Some("20050423"));
        let subs: Vec<(&str, bool)> = info
            .subtitles
            .iter()
            .map(|s| (s.language.as_str(), s.auto))
            .collect();
        // The uploader's subtitles, then the original captions, then their
        // translations; live chat is no subtitle.
        assert_eq!(
            subs,
            [
                ("de", false),
                ("en", false),
                ("en-orig", true),
                ("en", true),
                ("tr", true)
            ]
        );
        assert_eq!(info.subtitles[1].name.as_deref(), Some("English"));
        assert_eq!(info.extractor.as_deref(), Some("Youtube"));
        assert_eq!(info.duration_ms, Some(19_000));
        let ids: Vec<&str> = info.videos.iter().map(|v| v.id.as_str()).collect();
        // 240p: H.264 over AV1 and VP9, direct over HLS, then the higher bit
        // rate of the two H.264 formats. 144p likewise. Then audio only.
        assert_eq!(ids, ["133", "160", "140", "251"]);
        let best = &info.videos[0];
        assert_eq!(best.height, Some(240));
        assert_eq!(best.frame_rate.as_deref(), Some("15"));
        assert!(!best.has_audio && !best.audio_only);
        assert_eq!(best.estimated_size, Some(433_081));
        assert_eq!(best.codecs.as_deref(), Some("avc1.4D400C"));
        assert!(info.videos[2].audio_only);

        // DRC copies are dropped; AAC first and marked as the default.
        let audio: Vec<(&str, bool)> = info
            .audios
            .iter()
            .map(|a| (a.id.as_str(), a.default))
            .collect();
        assert_eq!(audio, [("140", true), ("251", false)]);
        assert_eq!(info.audios[0].bandwidth, Some(129_796));
    }

    fn info(formats: serde_json::Value) -> serde_json::Value {
        json!({"_type": "video", "title": "T", "duration": 10, "formats": formats})
    }

    #[test]
    fn keeps_formats_with_their_own_sound_apart() {
        let info = parse(
            info(json!([
                {"format_id": "18", "vcodec": "avc1.42001E", "acodec": "mp4a.40.2", "height": 360, "width": 640, "tbr": 500},
                {"format_id": "134", "vcodec": "avc1.4d401e", "acodec": "none", "height": 360, "tbr": 300},
                {"format_id": "140", "vcodec": "none", "acodec": "mp4a.40.2", "abr": 128},
            ]))
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        let videos: Vec<(&str, bool)> = info
            .videos
            .iter()
            .map(|v| (v.id.as_str(), v.has_audio))
            .collect();
        assert_eq!(videos, [("134", false), ("18", true), ("140", true)]);
        assert_eq!(
            info.videos[1].codecs.as_deref(),
            Some("avc1.42001E, mp4a.40.2")
        );
        // Size from the bit rate when none is given: 500 kbit/s for 10 s.
        assert_eq!(info.videos[1].estimated_size, Some(625_000));
    }

    #[test]
    fn languages_main_one_first() {
        let info = parse(
            info(json!([
                {"format_id": "137", "vcodec": "avc1", "acodec": "none", "height": 1080},
                {"format_id": "140-0", "vcodec": "none", "acodec": "mp4a.40.2", "abr": 128, "language": "es", "language_preference": -1},
                {"format_id": "140-1", "vcodec": "none", "acodec": "mp4a.40.2", "abr": 128, "language": "en", "language_preference": 10},
                {"format_id": "251-1", "vcodec": "none", "acodec": "opus", "abr": 140, "language": "en", "language_preference": 10},
            ]))
            .to_string()
            .as_bytes(),
        )
        .unwrap();
        let audio: Vec<&str> = info.audios.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(audio, ["140-1", "251-1", "140-0"]);
        assert!(info.audios[0].default);
        // Audio only is offered in the main language.
        let only: Vec<&str> = info
            .videos
            .iter()
            .filter(|v| v.audio_only)
            .map(|v| v.id.as_str())
            .collect();
        assert_eq!(only, ["140-1", "251-1"]);
    }

    #[test]
    fn refuses_live_playlists_and_drm() {
        let live = json!({"_type": "video", "is_live": true, "formats": []});
        assert!(matches!(
            parse(live.to_string().as_bytes()),
            Err(DownloadError::LiveStream)
        ));
        let upcoming = json!({"_type": "video", "live_status": "is_upcoming", "formats": []});
        assert!(matches!(
            parse(upcoming.to_string().as_bytes()),
            Err(DownloadError::LiveStream)
        ));
        let playlist = json!({"_type": "playlist", "entries": []});
        assert!(matches!(
            parse(playlist.to_string().as_bytes()),
            Err(DownloadError::UnsupportedMedia(_))
        ));
        let drm =
            info(json!([{"format_id": "1", "vcodec": "avc1", "height": 720, "has_drm": true}]));
        assert!(matches!(
            parse(drm.to_string().as_bytes()),
            Err(DownloadError::Protected)
        ));
        let maybe =
            info(json!([{"format_id": "1", "vcodec": "avc1", "height": 720, "has_drm": "maybe"}]));
        assert!(matches!(
            parse(maybe.to_string().as_bytes()),
            Err(DownloadError::Protected)
        ));
        // Protected formats next to free ones are only left out.
        let mixed = info(json!([
            {"format_id": "1", "vcodec": "avc1", "height": 720, "has_drm": true},
            {"format_id": "2", "vcodec": "avc1", "height": 480, "has_drm": false},
        ]));
        let ids: Vec<String> = parse(mixed.to_string().as_bytes())
            .unwrap()
            .videos
            .into_iter()
            .map(|v| v.id)
            .collect();
        assert_eq!(ids, ["2"]);
    }

    #[test]
    fn format_ids_that_are_not_plain_tokens_are_dropped() {
        let odd = info(json!([
            {"format_id": "1+2", "vcodec": "avc1", "height": 720},
            {"format_id": "a/b", "vcodec": "avc1", "height": 480},
            {"format_id": "ok-1", "vcodec": "avc1", "height": 360},
        ]));
        let ids: Vec<String> = parse(odd.to_string().as_bytes())
            .unwrap()
            .videos
            .into_iter()
            .map(|v| v.id)
            .collect();
        assert_eq!(ids, ["ok-1"]);
    }

    #[test]
    fn bad_output_is_an_extractor_error() {
        assert!(matches!(parse(b"null"), Err(DownloadError::Extractor(_))));
        assert!(matches!(parse(b"{"), Err(DownloadError::Extractor(_))));
        let title = json!({"title": "a\u{0007}b", "formats": [{"format_id": "1", "vcodec": "avc1", "height": 1}]});
        assert_eq!(
            parse(title.to_string().as_bytes())
                .unwrap()
                .title
                .as_deref(),
            Some("ab")
        );
    }
}
