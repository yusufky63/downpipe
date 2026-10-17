//! From a manifest URL to what can be downloaded: the qualities a stream
//! offers ([`MediaOptions`]), and for a choice the tracks and fragments to
//! fetch ([`Plan`]).

use std::time::Duration;

use futures_util::StreamExt;
use reqwest::header::HeaderMap;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use url::Url;

use super::dash::{self, ContentKind};
use super::hls::{self, MasterPlaylist, MediaPlaylist, Playlist, Rendition, Variant};
use super::{
    AudioOption, Container, MediaKind, MediaOptions, MediaSelection, Segment, TrackRef,
    VideoOption, quality_label,
};
use crate::error::status_error;
use crate::{DownloadError, http, retry};

/// Manifests are text; a few megabytes covers hours of segments.
pub(crate) const MAX_MANIFEST_BYTES: usize = 8 * 1024 * 1024;
const FETCH_ATTEMPTS: u32 = 4;
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// What a track is for once the download is assembled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    /// Video with its sound inside: one file, nothing to merge.
    Muxed,
    /// Video without sound; an [`TrackRole::Audio`] track goes with it.
    Video,
    Audio,
}

/// One stream to download: an optional initialization section and the
/// fragments that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub role: TrackRole,
    pub init: Option<Segment>,
    pub segments: Vec<Segment>,
    pub container: Container,
    /// From the announced bandwidth and the duration, for progress before
    /// the first fragments arrive.
    pub estimated_bytes: Option<u64>,
}

impl Track {
    /// Init section first, then the segments: the order they are joined in.
    pub fn fragments(&self) -> impl Iterator<Item = &Segment> {
        self.init.iter().chain(self.segments.iter())
    }

    pub fn fragment_count(&self) -> usize {
        self.segments.len() + usize::from(self.init.is_some())
    }
}

/// What to download for one choice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// One track, or video and audio to merge.
    pub tracks: Vec<Track>,
    /// "1080p" and the like, for the download list.
    pub quality: String,
    /// The tracks actually picked, so a resume picks them again.
    pub selection: MediaSelection,
}

/// A fetched manifest.
#[derive(Debug, Clone)]
pub(crate) enum Source {
    HlsMaster { master: MasterPlaylist },
    // Boxed: a media playlist is much bigger than the other variants.
    HlsMedia { playlist: Box<MediaPlaylist> },
    Dash { manifest: dash::Manifest },
}

/// Fetches manifest text with the download's headers, retrying transient
/// failures, and stops early when the download is paused.
#[derive(Debug, Clone)]
pub(crate) struct Fetcher {
    pub client: Client,
    pub headers: HeaderMap,
    pub stop: CancellationToken,
    pub retry_base: Duration,
    pub retry_max: Duration,
}

impl Fetcher {
    /// The text and the URL it came from after redirects, which relative
    /// URIs in it resolve against. `Ok(None)`: stopped meanwhile.
    pub async fn text(&self, url: &Url) -> Result<Option<(String, Url)>, DownloadError> {
        let mut last = None;
        for attempt in 0..FETCH_ATTEMPTS {
            let err = match self.try_text(url).await {
                Ok(found) => return Ok(found),
                Err(e) if e.is_retryable() => e,
                Err(e) => return Err(e),
            };
            let delay = err
                .retry_after()
                .unwrap_or_else(|| retry::backoff(attempt, self.retry_base, self.retry_max));
            tokio::select! {
                biased;
                () = self.stop.cancelled() => return Ok(None),
                () = tokio::time::sleep(delay) => {}
            }
            last = Some(err);
        }
        Err(DownloadError::RetriesExhausted {
            attempts: FETCH_ATTEMPTS,
            last: Box::new(last.unwrap_or(DownloadError::PrematureEnd)),
        })
    }

    /// The bytes of a small resource (an AES key), with the same rules.
    pub async fn bytes(&self, url: &Url, max: usize) -> Result<Option<Vec<u8>>, DownloadError> {
        let mut last = None;
        for attempt in 0..FETCH_ATTEMPTS {
            let err = match self.try_bytes(url, max).await {
                Ok(found) => return Ok(found.map(|(bytes, _)| bytes)),
                Err(e) if e.is_retryable() => e,
                Err(e) => return Err(e),
            };
            let delay = err
                .retry_after()
                .unwrap_or_else(|| retry::backoff(attempt, self.retry_base, self.retry_max));
            tokio::select! {
                biased;
                () = self.stop.cancelled() => return Ok(None),
                () = tokio::time::sleep(delay) => {}
            }
            last = Some(err);
        }
        Err(DownloadError::RetriesExhausted {
            attempts: FETCH_ATTEMPTS,
            last: Box::new(last.unwrap_or(DownloadError::PrematureEnd)),
        })
    }

    async fn try_text(&self, url: &Url) -> Result<Option<(String, Url)>, DownloadError> {
        let Some((bytes, final_url)) = self.try_bytes(url, MAX_MANIFEST_BYTES).await? else {
            return Ok(None);
        };
        let text = String::from_utf8(bytes)
            .map_err(|_| DownloadError::InvalidManifest("the manifest is not UTF-8 text".into()))?;
        Ok(Some((text, final_url)))
    }

    async fn try_bytes(
        &self,
        url: &Url,
        max: usize,
    ) -> Result<Option<(Vec<u8>, Url)>, DownloadError> {
        let request = http::plain_get(&self.client, url, &self.headers).timeout(FETCH_TIMEOUT);
        let resp = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Ok(None),
            r = request.send() => r.map_err(|e| DownloadError::from_reqwest(&e))?,
        };
        let status = resp.status();
        if status != StatusCode::OK {
            return Err(status_error(status.as_u16(), http::retry_after(&resp)));
        }
        if resp.content_length().is_some_and(|len| len > max as u64) {
            return Err(DownloadError::InvalidManifest(
                "the manifest is too large".into(),
            ));
        }
        let final_url = resp.url().clone();
        let mut body = Vec::new();
        let mut stream = resp.bytes_stream();
        loop {
            let chunk = tokio::select! {
                biased;
                () = self.stop.cancelled() => return Ok(None),
                c = stream.next() => c,
            };
            match chunk {
                None => break,
                Some(Err(e)) => return Err(DownloadError::from_reqwest(&e)),
                Some(Ok(chunk)) => {
                    if body.len() + chunk.len() > max {
                        return Err(DownloadError::InvalidManifest(
                            "the manifest is too large".into(),
                        ));
                    }
                    body.extend_from_slice(&chunk);
                }
            }
        }
        Ok(Some((body, final_url)))
    }
}

/// Fetch and parse the manifest a download starts from.
pub(crate) async fn load(
    fetcher: &Fetcher,
    url: &Url,
    kind: MediaKind,
) -> Result<Option<Source>, DownloadError> {
    let Some((text, base)) = fetcher.text(url).await? else {
        return Ok(None);
    };
    Ok(Some(match kind {
        MediaKind::Hls => match hls::parse(&text, &base)? {
            Playlist::Master(master) => Source::HlsMaster { master },
            Playlist::Media(playlist) => Source::HlsMedia {
                playlist: Box::new(playlist),
            },
        },
        MediaKind::Dash => Source::Dash {
            manifest: dash::parse(&text, &base)?,
        },
    }))
}

const VIDEO_CODECS: &[&str] = &[
    "avc", "hvc", "hev", "vp8", "vp9", "vp09", "av01", "dvh", "dvav",
];

fn audio_only(codecs: Option<&str>) -> bool {
    codecs.is_some_and(|c| {
        !c.split(',').any(|codec| {
            VIDEO_CODECS
                .iter()
                .any(|v| codec.trim().to_ascii_lowercase().starts_with(v))
        })
    })
}

/// Variants that are video, best first, with their position in the master.
fn ranked_variants(master: &MasterPlaylist) -> Vec<(usize, &Variant)> {
    let mut ranked: Vec<(usize, &Variant)> = master.variants.iter().enumerate().collect();
    ranked.sort_by_key(|(_, v)| {
        std::cmp::Reverse((
            v.resolution.map_or(0, |(_, h)| h),
            v.average_bandwidth.or(v.bandwidth).unwrap_or(0),
        ))
    });
    ranked
}

/// Audio renditions with their own playlist, in the master's order.
fn separate_audio(master: &MasterPlaylist) -> Vec<(usize, &Rendition)> {
    master
        .audio
        .iter()
        .enumerate()
        .filter(|(_, r)| r.uri.is_some())
        .collect()
}

fn variant_has_own_audio(master: &MasterPlaylist, variant: &Variant) -> bool {
    match &variant.audio_group {
        None => true,
        Some(group) => !master
            .audio
            .iter()
            .any(|r| &r.group_id == group && r.uri.is_some()),
    }
}

fn duration_ms(segments: &[Segment]) -> u64 {
    segments.iter().map(|s| s.duration_ms).sum()
}

fn estimate(bandwidth: Option<u64>, duration_ms: Option<u64>) -> Option<u64> {
    let bits = u128::from(bandwidth?) * u128::from(duration_ms?) / 1000;
    u64::try_from(bits / 8).ok().filter(|b| *b > 0)
}

/// What a fetched manifest offers, best quality first.
pub(crate) fn options(source: &Source) -> MediaOptions {
    match source {
        Source::HlsMaster { master } => MediaOptions {
            kind: MediaKind::Hls,
            videos: ranked_variants(master)
                .into_iter()
                .map(|(i, v)| VideoOption {
                    id: format!("v{i}"),
                    width: v.resolution.map(|(w, _)| w),
                    height: v.resolution.map(|(_, h)| h),
                    bandwidth: v.bandwidth,
                    codecs: v.codecs.clone(),
                    frame_rate: v.frame_rate.clone(),
                    has_audio: variant_has_own_audio(master, v),
                    audio_group: v.audio_group.clone(),
                    audio_only: audio_only(v.codecs.as_deref()),
                    estimated_size: None,
                })
                .collect(),
            audios: separate_audio(master)
                .into_iter()
                .map(|(i, r)| AudioOption {
                    id: format!("a{i}"),
                    name: r.name.clone(),
                    language: r.language.clone(),
                    bandwidth: None,
                    codecs: None,
                    default: r.default,
                    group: Some(r.group_id.clone()),
                })
                .collect(),
            duration_ms: None,
        },
        Source::HlsMedia { playlist } => MediaOptions {
            kind: MediaKind::Hls,
            videos: vec![VideoOption {
                id: "v0".into(),
                width: None,
                height: None,
                bandwidth: None,
                codecs: None,
                frame_rate: None,
                has_audio: true,
                audio_group: None,
                audio_only: false,
                estimated_size: None,
            }],
            audios: Vec::new(),
            duration_ms: Some(duration_ms(&playlist.segments)),
        },
        Source::Dash { manifest } => {
            let audios = dash_audio(manifest);
            let mut videos: Vec<&dash::Representation> = manifest
                .representations
                .iter()
                .filter(|r| r.kind == ContentKind::Video)
                .collect();
            videos.sort_by_key(|r| {
                std::cmp::Reverse((r.height.unwrap_or(0), r.bandwidth.unwrap_or(0)))
            });
            MediaOptions {
                kind: MediaKind::Dash,
                videos: videos
                    .into_iter()
                    .map(|r| VideoOption {
                        id: r.id.clone(),
                        width: r.width,
                        height: r.height,
                        bandwidth: r.bandwidth,
                        codecs: r.codecs.clone(),
                        frame_rate: r.frame_rate.clone(),
                        // With no audio adaptation set, any sound is inside.
                        has_audio: audios.is_empty(),
                        audio_group: None,
                        audio_only: false,
                        estimated_size: estimate(r.bandwidth, manifest.duration_ms),
                    })
                    .collect(),
                audios: audios
                    .into_iter()
                    .map(|r| AudioOption {
                        id: r.id.clone(),
                        name: r.label.clone(),
                        language: r.language.clone(),
                        bandwidth: r.bandwidth,
                        codecs: r.codecs.clone(),
                        default: r.main,
                        group: None,
                    })
                    .collect(),
                duration_ms: manifest.duration_ms,
            }
        }
    }
}

/// Audio representations, main ones first, then by bandwidth.
fn dash_audio(manifest: &dash::Manifest) -> Vec<&dash::Representation> {
    let mut audios: Vec<&dash::Representation> = manifest
        .representations
        .iter()
        .filter(|r| r.kind == ContentKind::Audio)
        .collect();
    audios.sort_by_key(|r| std::cmp::Reverse((r.main, r.bandwidth.unwrap_or(0))));
    audios
}

/// The option a stored choice means now: the same id, else the same height
/// with the closest bandwidth, else the best one.
fn pick_video<'a>(videos: &'a [VideoOption], wanted: Option<&TrackRef>) -> Option<&'a VideoOption> {
    let Some(wanted) = wanted else {
        return videos.first();
    };
    videos
        .iter()
        .find(|v| v.id == wanted.id && (wanted.height.is_none() || v.height == wanted.height))
        .or_else(|| {
            videos
                .iter()
                .filter(|v| wanted.height.is_some() && v.height == wanted.height)
                .min_by_key(|v| {
                    v.bandwidth
                        .unwrap_or(0)
                        .abs_diff(wanted.bandwidth.unwrap_or(0))
                })
        })
        .or_else(|| videos.first())
}

/// The audio that goes with a video: the stored choice when it fits,
/// else one in the same language, else the default, else the first.
fn pick_audio<'a>(
    audios: &'a [AudioOption],
    group: Option<&str>,
    wanted: Option<&TrackRef>,
) -> Option<&'a AudioOption> {
    let fitting: Vec<&AudioOption> = audios
        .iter()
        .filter(|a| group.is_none() || a.group.as_deref() == group)
        .collect();
    let wanted_lang = wanted.and_then(|w| w.language.as_deref());
    wanted
        .and_then(|w| fitting.iter().find(|a| a.id == w.id).copied())
        .or_else(|| {
            wanted_lang.and_then(|lang| {
                fitting
                    .iter()
                    .find(|a| a.language.as_deref() == Some(lang))
                    .copied()
            })
        })
        .or_else(|| fitting.iter().find(|a| a.default).copied())
        .or_else(|| fitting.first().copied())
}

fn video_ref(v: &VideoOption) -> TrackRef {
    TrackRef {
        id: v.id.clone(),
        height: v.height,
        bandwidth: v.bandwidth,
        language: None,
    }
}

fn audio_ref(a: &AudioOption) -> TrackRef {
    TrackRef {
        id: a.id.clone(),
        height: None,
        bandwidth: a.bandwidth,
        language: a.language.clone(),
    }
}

fn audio_label(a: &AudioOption) -> String {
    a.name
        .clone()
        .or_else(|| a.language.clone())
        .unwrap_or_else(|| "Audio".to_string())
}

/// Index of an `"v3"` / `"a1"` id into the master's lists.
fn index_of(id: &str, prefix: char) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

/// Guess the container from an HLS playlist: an init section means
/// fragmented MP4, otherwise the segment extension says it, and plain
/// transport streams are the default.
fn hls_container(playlist: &MediaPlaylist, audio: bool) -> Container {
    let from_path = |s: &Segment| Container::from_path(s.url.path());
    let guessed = playlist
        .init
        .as_ref()
        .and_then(from_path)
        .or_else(|| playlist.segments.first().and_then(from_path));
    match (playlist.init.is_some(), guessed) {
        (true, Some(Container::WebM)) => Container::WebM,
        (true, _) if audio => Container::M4a,
        (true, _) => Container::Mp4,
        (false, Some(Container::Mp4 | Container::M4a)) if audio => Container::M4a,
        (false, Some(c)) => c,
        (false, None) => Container::Ts,
    }
}

fn hls_track(playlist: MediaPlaylist, role: TrackRole, bandwidth: Option<u64>) -> Track {
    let container = hls_container(&playlist, role == TrackRole::Audio);
    let estimated_bytes = estimate(bandwidth, Some(duration_ms(&playlist.segments)));
    Track {
        role,
        init: playlist.init,
        segments: playlist.segments,
        container,
        estimated_bytes,
    }
}

async fn media_playlist(
    fetcher: &Fetcher,
    url: &Url,
) -> Result<Option<MediaPlaylist>, DownloadError> {
    let Some((text, base)) = fetcher.text(url).await? else {
        return Ok(None);
    };
    match hls::parse(&text, &base)? {
        Playlist::Media(playlist) => Ok(Some(playlist)),
        Playlist::Master(_) => Err(DownloadError::InvalidManifest(
            "a quality playlist is itself a master playlist".into(),
        )),
    }
}

fn dash_container(r: &dash::Representation) -> Container {
    let webm = r.mime_type.as_deref().is_some_and(|m| m.ends_with("/webm"));
    match (webm, r.kind) {
        (true, _) => Container::WebM,
        (false, ContentKind::Audio) => Container::M4a,
        (false, _) => r
            .segments
            .first()
            .and_then(|s| Container::from_path(s.url.path()))
            .filter(|c| *c == Container::WebM)
            .unwrap_or(Container::Mp4),
    }
}

fn dash_track(r: &dash::Representation, role: TrackRole, duration: Option<u64>) -> Track {
    Track {
        role,
        init: r.init.clone(),
        segments: r.segments.clone(),
        container: dash_container(r),
        estimated_bytes: estimate(r.bandwidth, duration),
    }
}

/// Turn a choice into tracks, fetching quality playlists as needed.
/// `Ok(None)`: stopped meanwhile.
pub(crate) async fn plan(
    fetcher: &Fetcher,
    source: Source,
    selection: &MediaSelection,
) -> Result<Option<Plan>, DownloadError> {
    let opts = options(&source);
    match source {
        Source::HlsMedia { playlist } => Ok(Some(Plan {
            tracks: vec![hls_track(*playlist, TrackRole::Muxed, None)],
            quality: "Default".into(),
            selection: MediaSelection::default(),
        })),
        Source::HlsMaster { master } => {
            let video = pick_video(&opts.videos, selection.video.as_ref()).ok_or_else(|| {
                DownloadError::InvalidManifest("the master playlist lists no qualities".into())
            })?;
            let variant = index_of(&video.id, 'v')
                .and_then(|i| master.variants.get(i))
                .ok_or_else(|| {
                    DownloadError::InvalidManifest("the chosen quality is gone".into())
                })?;
            let Some(playlist) = media_playlist(fetcher, &variant.uri).await? else {
                return Ok(None);
            };
            let mut quality =
                quality_label(video.height, video.frame_rate.as_deref(), video.bandwidth);
            let role = if video.audio_only {
                TrackRole::Audio
            } else if video.has_audio {
                TrackRole::Muxed
            } else {
                TrackRole::Video
            };
            let mut tracks = vec![hls_track(
                playlist,
                role,
                variant.average_bandwidth.or(variant.bandwidth),
            )];
            let mut chosen = MediaSelection {
                video: Some(video_ref(video)),
                audio: None,
            };
            if role == TrackRole::Video
                && let Some(audio) = pick_audio(
                    &opts.audios,
                    video.audio_group.as_deref(),
                    selection.audio.as_ref(),
                )
            {
                let rendition = index_of(&audio.id, 'a').and_then(|i| master.audio.get(i));
                if let Some(uri) = rendition.and_then(|r| r.uri.as_ref()) {
                    let Some(playlist) = media_playlist(fetcher, uri).await? else {
                        return Ok(None);
                    };
                    tracks.push(hls_track(playlist, TrackRole::Audio, None));
                    if opts
                        .audios
                        .iter()
                        .filter(|a| a.group == audio.group)
                        .count()
                        > 1
                    {
                        quality = format!("{quality} · {}", audio_label(audio));
                    }
                    chosen.audio = Some(audio_ref(audio));
                }
            }
            if tracks.len() == 1 && role == TrackRole::Video {
                // No separate audio after all: whatever sound there is, is inside.
                tracks[0].role = TrackRole::Muxed;
            }
            Ok(Some(Plan {
                tracks,
                quality,
                selection: chosen,
            }))
        }
        Source::Dash { manifest } => {
            let duration = manifest.duration_ms;
            let find = |id: &str| manifest.representations.iter().find(|r| r.id == id);
            let video = pick_video(&opts.videos, selection.video.as_ref());
            let audio = pick_audio(&opts.audios, None, selection.audio.as_ref());
            let mut tracks = Vec::new();
            let mut chosen = MediaSelection::default();
            let mut quality = "Audio".to_string();
            if let Some(v) = video
                && let Some(rep) = find(&v.id)
            {
                let role = if audio.is_some() {
                    TrackRole::Video
                } else {
                    TrackRole::Muxed
                };
                tracks.push(dash_track(rep, role, duration));
                quality = quality_label(v.height, v.frame_rate.as_deref(), v.bandwidth);
                chosen.video = Some(video_ref(v));
            }
            if let Some(a) = audio
                && let Some(rep) = find(&a.id)
            {
                tracks.push(dash_track(rep, TrackRole::Audio, duration));
                let languages = opts
                    .audios
                    .iter()
                    .filter(|o| o.language != a.language)
                    .count();
                if languages > 0 {
                    quality = format!("{quality} · {}", audio_label(a));
                }
                chosen.audio = Some(audio_ref(a));
            }
            if tracks.is_empty() {
                return Err(DownloadError::UnsupportedMedia(
                    "the manifest has neither video nor audio".into(),
                ));
            }
            Ok(Some(Plan {
                tracks,
                quality,
                selection: chosen,
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).unwrap()
    }

    const MASTER: &str = "#EXTM3U
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=YES,URI=\"audio/en.m3u8\"
#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"Deutsch\",LANGUAGE=\"de\",URI=\"audio/de.m3u8\"
#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,AUDIO=\"aud\"
360/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=5000000,RESOLUTION=1920x1080,FRAME-RATE=60,AUDIO=\"aud\"
1080/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,AUDIO=\"aud\"
720/index.m3u8
";

    fn master() -> Source {
        match hls::parse(MASTER, &url("https://v.example.net/s/master.m3u8")).unwrap() {
            Playlist::Master(master) => Source::HlsMaster { master },
            Playlist::Media(_) => unreachable!(),
        }
    }

    #[test]
    fn hls_options_rank_qualities_and_list_audio() {
        let opts = options(&master());
        let heights: Vec<_> = opts.videos.iter().map(|v| v.height).collect();
        assert_eq!(heights, [Some(1080), Some(720), Some(360)]);
        assert_eq!(opts.videos[0].id, "v1");
        assert!(!opts.videos[0].has_audio);
        assert_eq!(opts.videos[0].audio_group.as_deref(), Some("aud"));
        assert_eq!(opts.audios.len(), 2);
        assert!(opts.audios[0].default);
        assert_eq!(opts.audios[1].language.as_deref(), Some("de"));
    }

    #[test]
    fn stored_choices_are_found_again() {
        let opts = options(&master());
        let wanted = TrackRef {
            id: "v9".into(),
            height: Some(720),
            bandwidth: Some(2_400_000),
            language: None,
        };
        assert_eq!(pick_video(&opts.videos, Some(&wanted)).unwrap().id, "v2");
        assert_eq!(pick_video(&opts.videos, None).unwrap().height, Some(1080));
        let german = TrackRef {
            id: "gone".into(),
            language: Some("de".into()),
            ..TrackRef::default()
        };
        assert_eq!(
            pick_audio(&opts.audios, Some("aud"), Some(&german))
                .unwrap()
                .id,
            "a1"
        );
        assert_eq!(
            pick_audio(&opts.audios, Some("aud"), None).unwrap().id,
            "a0"
        );
        assert!(pick_audio(&opts.audios, Some("other"), None).is_none());
    }

    #[test]
    fn audio_only_variants_are_recognized() {
        assert!(audio_only(Some("mp4a.40.2")));
        assert!(!audio_only(Some("avc1.64001f,mp4a.40.2")));
        assert!(!audio_only(None));
    }

    #[test]
    fn containers_come_from_init_and_extensions() {
        let base = url("https://v.example.net/a/index.m3u8");
        let parse = |text: &str| match hls::parse(text, &base).unwrap() {
            Playlist::Media(p) => p,
            Playlist::Master(_) => unreachable!(),
        };
        let ts = parse("#EXTM3U\n#EXTINF:4,\na.ts\n#EXT-X-ENDLIST\n");
        assert_eq!(hls_container(&ts, false), Container::Ts);
        let fmp4 =
            parse("#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:4,\na.m4s\n#EXT-X-ENDLIST\n");
        assert_eq!(hls_container(&fmp4, false), Container::Mp4);
        assert_eq!(hls_container(&fmp4, true), Container::M4a);
        let bare = parse("#EXTM3U\n#EXTINF:4,\nseg?id=1\n#EXT-X-ENDLIST\n");
        assert_eq!(hls_container(&bare, false), Container::Ts);
        let aac = parse("#EXTM3U\n#EXTINF:4,\na.aac\n#EXT-X-ENDLIST\n");
        assert_eq!(hls_container(&aac, true), Container::Aac);
    }

    #[test]
    fn estimates_need_bandwidth_and_duration() {
        assert_eq!(estimate(Some(8_000_000), Some(10_000)), Some(10_000_000));
        assert_eq!(estimate(None, Some(10_000)), None);
        assert_eq!(estimate(Some(1), None), None);
    }

    const MPD: &str = r#"<?xml version="1.0"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="PT8S">
  <Period>
    <AdaptationSet mimeType="video/mp4">
      <SegmentTemplate media="$RepresentationID$/$Number$.m4s" initialization="$RepresentationID$/init.mp4" duration="4" startNumber="1"/>
      <Representation id="v720" bandwidth="2000000" width="1280" height="720"/>
      <Representation id="v1080" bandwidth="4000000" width="1920" height="1080"/>
    </AdaptationSet>
    <AdaptationSet mimeType="audio/mp4" lang="en">
      <Role schemeIdUri="urn:mpeg:dash:role:2011" value="main"/>
      <SegmentTemplate media="$RepresentationID$/$Number$.m4s" initialization="$RepresentationID$/init.mp4" duration="4"/>
      <Representation id="a128" bandwidth="128000"/>
    </AdaptationSet>
  </Period>
</MPD>"#;

    #[tokio::test]
    async fn dash_plans_video_and_audio() {
        let manifest = dash::parse(MPD, &url("https://v.example.net/d/manifest.mpd")).unwrap();
        let source = Source::Dash { manifest };
        let opts = options(&source);
        assert_eq!(opts.videos[0].id, "v1080");
        assert_eq!(opts.videos[0].estimated_size, Some(4_000_000));
        assert!(!opts.videos[0].has_audio);
        let fetcher = Fetcher {
            client: Client::new(),
            headers: HeaderMap::new(),
            stop: CancellationToken::new(),
            retry_base: Duration::from_millis(1),
            retry_max: Duration::from_millis(1),
        };
        let wanted = MediaSelection {
            video: Some(TrackRef {
                id: "v720".into(),
                height: Some(720),
                ..TrackRef::default()
            }),
            audio: None,
        };
        let plan = plan(&fetcher, source, &wanted).await.unwrap().unwrap();
        assert_eq!(plan.quality, "720p");
        assert_eq!(plan.tracks.len(), 2);
        assert_eq!(plan.tracks[0].role, TrackRole::Video);
        assert_eq!(plan.tracks[0].container, Container::Mp4);
        assert_eq!(plan.tracks[0].fragment_count(), 3);
        assert_eq!(plan.tracks[1].role, TrackRole::Audio);
        assert_eq!(plan.tracks[1].container, Container::M4a);
        assert_eq!(plan.selection.audio.unwrap().id, "a128");
    }
}
