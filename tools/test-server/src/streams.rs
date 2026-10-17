//! HLS and DASH streams with deterministic, checkable fragments.
//!
//! The bytes are not playable media: each fragment is a function of its
//! track and number (see [`fragment`]), so tests can predict the joined file.
//! The query string of a manifest request carries the options and is copied
//! onto every URI the manifest lists, so fragments see them too:
//!
//! | parameter        | effect                                                      |
//! |------------------|-------------------------------------------------------------|
//! | `segments=N`     | fragments per track (default 6)                             |
//! | `size=N`         | bytes per fragment before encryption (default 40000)        |
//! | `split=1`        | video and audio as separate tracks, fragmented MP4 with init |
//! | `aes=1`          | HLS: AES-128, explicit IV on video, sequence-number IV else |
//! | `live=1`         | HLS without `#EXT-X-ENDLIST`, DASH `type="dynamic"`         |
//! | `drm=1`          | HLS `SAMPLE-AES`, DASH `ContentProtection`                  |
//! | `fail_seg=I`     | fragment I answers 500 for the first `fail_times` requests  |
//! | `rate=N`         | at most N bytes/s per fragment response                     |
//! | `flat=1`         | HLS: quality and audio playlists beside the master          |
//!
//! `/hls/master.m3u8`, `/hls/{track}/index.m3u8` (or `/hls/{track}.m3u8`
//! with `flat=1`, the layout of many CDNs), `/hls/{track}/{file}`,
//! `/hls/key.bin`, `/dash/manifest.mpd` and `/dash/{track}/{file}`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use aes::Aes128;
use axum::Router;
use axum::extract::{Path, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use cbc::cipher::block_padding::Pkcs7;
use cbc::cipher::{BlockEncryptMut, KeyIvInit};
use futures_util::StreamExt;

use crate::{ServerState, byte_at};

/// The AES-128 key `/hls/key.bin` serves.
pub const KEY: [u8; 16] = *b"downpipe-test-ky";
/// Explicit IV of encrypted video playlists.
pub const VIDEO_IV: [u8; 16] = [7; 16];
/// `#EXT-X-MEDIA-SEQUENCE` of every HLS playlist, so implicit IVs are not zero.
pub const MEDIA_SEQUENCE: u64 = 5;
/// Bytes of an init section.
pub const INIT_SIZE: u64 = 1000;
const SEGMENT_SECONDS: u64 = 4;

/// Options of one request, from its query string.
#[derive(Debug, Clone)]
pub struct StreamOptions {
    pub segments: u64,
    pub size: u64,
    pub split: bool,
    pub aes: bool,
    pub live: bool,
    pub drm: bool,
    pub fail_seg: Option<u64>,
    pub fail_times: u64,
    pub rate: Option<u64>,
    pub flat: bool,
    /// The query as received, copied onto every URI.
    query: String,
}

impl StreamOptions {
    fn parse(query: Option<&str>) -> Self {
        let pairs: HashMap<&str, &str> = query
            .unwrap_or("")
            .split('&')
            .filter_map(|p| p.split_once('='))
            .collect();
        let num = |k: &str| pairs.get(k).and_then(|v| v.parse::<u64>().ok());
        let flag = |k: &str| pairs.get(k) == Some(&"1");
        Self {
            segments: num("segments").unwrap_or(6).clamp(1, 10_000),
            size: num("size").unwrap_or(40_000).clamp(1, 64 * 1024 * 1024),
            split: flag("split"),
            aes: flag("aes"),
            live: flag("live"),
            drm: flag("drm"),
            fail_seg: num("fail_seg"),
            fail_times: num("fail_times").unwrap_or(1),
            rate: num("rate"),
            flat: flag("flat"),
            query: query.unwrap_or("").to_string(),
        }
    }

    fn suffix(&self) -> String {
        if self.query.is_empty() {
            String::new()
        } else {
            format!("?{}", self.query)
        }
    }
}

/// Tracks of a stream: two video qualities, plus audio when split.
pub fn tracks(split: bool) -> Vec<&'static str> {
    if split {
        vec!["v0", "v1", "audio"]
    } else {
        vec!["v0", "v1"]
    }
}

fn seed(track: &str) -> u64 {
    match track {
        "v0" => 1,
        "v1" => 2,
        "audio" => 3,
        _ => 9,
    }
}

/// Plain bytes of fragment `index` (0-based) of `track`.
pub fn fragment(track: &str, index: u64, size: u64) -> Vec<u8> {
    let version = seed(track) * 1_000_000 + index;
    (0..size).map(|i| byte_at(i, version)).collect()
}

/// Plain bytes of a track's init section.
pub fn init(track: &str) -> Vec<u8> {
    (0..INIT_SIZE)
        .map(|i| byte_at(i, seed(track) * 1_000_000 + 999_999))
        .collect()
}

/// What a client should end up with for one track: init (when split) and
/// every fragment, decrypted.
pub fn joined(track: &str, opts_query: &str) -> Vec<u8> {
    let opts = StreamOptions::parse(Some(opts_query));
    let mut out = if opts.split { init(track) } else { Vec::new() };
    for i in 0..opts.segments {
        out.extend(fragment(track, i, opts.size));
    }
    out
}

/// IV of fragment `index`: explicit for video, its sequence number otherwise.
pub fn iv(track: &str, index: u64) -> [u8; 16] {
    if track.starts_with('v') {
        VIDEO_IV
    } else {
        u128::from(MEDIA_SEQUENCE + index).to_be_bytes()
    }
}

fn encrypt(plain: &[u8], iv: &[u8; 16]) -> Vec<u8> {
    let mut buf = plain.to_vec();
    buf.resize(plain.len() + 16, 0);
    let len = cbc::Encryptor::<Aes128>::new(&KEY.into(), iv.into())
        .encrypt_padded_mut::<Pkcs7>(&mut buf, plain.len())
        .map(<[u8]>::len)
        .unwrap_or(0);
    buf.truncate(len);
    buf
}

pub(crate) fn routes(router: Router<Arc<ServerState>>) -> Router<Arc<ServerState>> {
    router
        .route("/hls/master.m3u8", get(hls_master))
        .route("/hls/key.bin", get(hls_key))
        .route("/hls/{file}", get(hls_flat_media))
        .route("/hls/{track}/index.m3u8", get(hls_media))
        .route("/hls/{track}/{file}", get(hls_fragment))
        .route("/dash/manifest.mpd", get(dash_manifest))
        .route("/dash/{track}/{file}", get(dash_fragment))
}

fn text(body: String, mime: &'static str) -> Response {
    let mut resp = body.into_response();
    resp.headers_mut()
        .insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    resp
}

/// Where the master points for a track's playlist.
fn media_uri(track: &str, opts: &StreamOptions) -> String {
    let s = opts.suffix();
    if opts.flat {
        format!("{track}.m3u8{s}")
    } else {
        format!("{track}/index.m3u8{s}")
    }
}

async fn hls_master(
    State(state): State<Arc<ServerState>>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let opts = StreamOptions::parse(q.as_deref());
    state.record("/hls/master.m3u8", &headers, StatusCode::OK);
    text(master_playlist(&opts), "application/vnd.apple.mpegurl")
}

fn master_playlist(opts: &StreamOptions) -> String {
    let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-INDEPENDENT-SEGMENTS\n");
    if opts.drm {
        out.push_str(
            "#EXT-X-SESSION-KEY:METHOD=SAMPLE-AES,URI=\"skd://key\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n",
        );
    }
    let audio = if opts.split {
        out.push_str(&format!(
            "#EXT-X-MEDIA:TYPE=AUDIO,GROUP-ID=\"aud\",NAME=\"English\",LANGUAGE=\"en\",DEFAULT=YES,AUTOSELECT=YES,URI=\"{}\"\n",
            media_uri("audio", opts)
        ));
        ",AUDIO=\"aud\""
    } else {
        ""
    };
    out.push_str(&format!(
        "#EXT-X-STREAM-INF:BANDWIDTH=800000,RESOLUTION=640x360,CODECS=\"avc1.4d401e,mp4a.40.2\"{audio}\n{}\n",
        media_uri("v0", opts)
    ));
    out.push_str(&format!(
        "#EXT-X-STREAM-INF:BANDWIDTH=2500000,RESOLUTION=1280x720,CODECS=\"avc1.4d401f,mp4a.40.2\"{audio}\n{}\n",
        media_uri("v1", opts)
    ));
    out
}

async fn hls_media(
    State(state): State<Arc<ServerState>>,
    Path(track): Path<String>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let opts = StreamOptions::parse(q.as_deref());
    state.record(
        &format!("/hls/{track}/index.m3u8"),
        &headers,
        StatusCode::OK,
    );
    text(
        media_playlist(&track, &opts, ""),
        "application/vnd.apple.mpegurl",
    )
}

/// `/hls/{track}.m3u8`: a playlist beside the master, whose fragments stay
/// in the track's folder.
async fn hls_flat_media(
    State(state): State<Arc<ServerState>>,
    Path(file): Path<String>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let path = format!("/hls/{file}");
    let Some(track) = file
        .strip_suffix(".m3u8")
        .filter(|t| tracks(true).contains(t))
    else {
        state.record(&path, &headers, StatusCode::NOT_FOUND);
        return StatusCode::NOT_FOUND.into_response();
    };
    let opts = StreamOptions::parse(q.as_deref());
    state.record(&path, &headers, StatusCode::OK);
    text(
        media_playlist(track, &opts, &format!("{track}/")),
        "application/vnd.apple.mpegurl",
    )
}

/// A track's media playlist; `prefix` leads its fragment URIs to the track's folder.
fn media_playlist(track: &str, opts: &StreamOptions, prefix: &str) -> String {
    let s = opts.suffix();
    let ext = if opts.split { "m4s" } else { "ts" };
    let mut out = format!(
        "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:{SEGMENT_SECONDS}\n#EXT-X-MEDIA-SEQUENCE:{MEDIA_SEQUENCE}\n"
    );
    if !opts.live {
        out.push_str("#EXT-X-PLAYLIST-TYPE:VOD\n");
    }
    if opts.drm {
        out.push_str("#EXT-X-KEY:METHOD=SAMPLE-AES,URI=\"skd://key\",KEYFORMAT=\"com.apple.streamingkeydelivery\"\n");
    } else if opts.aes {
        let iv = if track.starts_with('v') {
            format!(",IV=0x{}", hex(&VIDEO_IV))
        } else {
            String::new()
        };
        out.push_str(&format!(
            "#EXT-X-KEY:METHOD=AES-128,URI=\"/hls/key.bin{s}\"{iv}\n"
        ));
    }
    if opts.split {
        out.push_str(&format!("#EXT-X-MAP:URI=\"{prefix}init.mp4{s}\"\n"));
    }
    for i in 0..opts.segments {
        out.push_str(&format!(
            "#EXTINF:{SEGMENT_SECONDS}.000,\n{prefix}seg-{i}.{ext}{s}\n"
        ));
    }
    if !opts.live {
        out.push_str("#EXT-X-ENDLIST\n");
    }
    out
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

async fn hls_key(State(state): State<Arc<ServerState>>, headers: HeaderMap) -> Response {
    state.record("/hls/key.bin", &headers, StatusCode::OK);
    KEY.to_vec().into_response()
}

async fn hls_fragment(
    State(state): State<Arc<ServerState>>,
    Path((track, file)): Path<(String, String)>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let opts = StreamOptions::parse(q.as_deref());
    serve_fragment(&state, "hls", &track, &file, &opts, &headers, 0).await
}

async fn dash_fragment(
    State(state): State<Arc<ServerState>>,
    Path((track, file)): Path<(String, String)>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let opts = StreamOptions::parse(q.as_deref());
    // DASH numbers start at 1.
    serve_fragment(&state, "dash", &track, &file, &opts, &headers, 1).await
}

async fn serve_fragment(
    state: &Arc<ServerState>,
    kind: &str,
    track: &str,
    file: &str,
    opts: &StreamOptions,
    headers: &HeaderMap,
    first_number: u64,
) -> Response {
    let path = format!("/{kind}/{track}/{file}");
    let encrypted = opts.aes && kind == "hls";
    let body = if file.starts_with("init.") {
        // A key before `#EXT-X-MAP` covers the init section too.
        if encrypted {
            encrypt(&init(track), &iv(track, 0))
        } else {
            init(track)
        }
    } else {
        let Some(index) = file
            .strip_prefix("seg-")
            .and_then(|r| r.split('.').next())
            .and_then(|n| n.parse::<u64>().ok())
            .and_then(|n| n.checked_sub(first_number))
            .filter(|i| *i < opts.segments)
        else {
            state.record(&path, headers, StatusCode::NOT_FOUND);
            return StatusCode::NOT_FOUND.into_response();
        };
        if opts.fail_seg == Some(index) {
            let count = {
                let mut seen = state.fragment_failures.lock();
                let n = seen.entry(path.clone()).or_insert(0);
                *n += 1;
                *n
            };
            if count <= opts.fail_times {
                state.record(&path, headers, StatusCode::INTERNAL_SERVER_ERROR);
                return StatusCode::INTERNAL_SERVER_ERROR.into_response();
            }
        }
        let plain = fragment(track, index, opts.size);
        if encrypted {
            encrypt(&plain, &iv(track, index))
        } else {
            plain
        }
    };
    state.record(&path, headers, StatusCode::OK);
    state
        .bytes_served
        .fetch_add(body.len() as u64, Ordering::SeqCst);
    let len = body.len();
    let mut resp = match opts.rate.filter(|r| *r > 0) {
        Some(rate) => {
            // Paced in small chunks, so a test can pause mid-fragment.
            let chunks: Vec<Vec<u8>> = body.chunks(4096).map(<[u8]>::to_vec).collect();
            let delay = Duration::from_secs_f64(4096.0 / rate as f64);
            let stream = futures_util::stream::iter(chunks).then(move |c| async move {
                tokio::time::sleep(delay).await;
                Ok::<_, std::io::Error>(bytes::Bytes::from(c))
            });
            Response::new(axum::body::Body::from_stream(stream))
        }
        None => Response::new(axum::body::Body::from(body)),
    };
    let h = resp.headers_mut();
    h.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(if file.ends_with(".ts") {
            "video/mp2t"
        } else {
            "video/mp4"
        }),
    );
    resp
}

async fn dash_manifest(
    State(state): State<Arc<ServerState>>,
    RawQuery(q): RawQuery,
    headers: HeaderMap,
) -> Response {
    let opts = StreamOptions::parse(q.as_deref());
    state.record("/dash/manifest.mpd", &headers, StatusCode::OK);
    // `&` must be escaped inside XML attributes.
    let s = opts.suffix().replace('&', "&amp;");
    let seconds = opts.segments * SEGMENT_SECONDS;
    let kind = if opts.live { "dynamic" } else { "static" };
    let protection = if opts.drm {
        "<ContentProtection schemeIdUri=\"urn:uuid:edef8ba9-79d6-4ace-a3c8-27dcd51d21ed\"/>"
    } else {
        ""
    };
    let template = format!(
        "<SegmentTemplate media=\"$RepresentationID$/seg-$Number$.m4s{s}\" initialization=\"$RepresentationID$/init.mp4{s}\" duration=\"{SEGMENT_SECONDS}\" timescale=\"1\" startNumber=\"1\"/>"
    );
    let audio = if opts.split {
        format!(
            "<AdaptationSet mimeType=\"audio/mp4\" codecs=\"mp4a.40.2\" lang=\"en\"><Role schemeIdUri=\"urn:mpeg:dash:role:2011\" value=\"main\"/>{template}<Representation id=\"audio\" bandwidth=\"128000\"/></AdaptationSet>"
        )
    } else {
        String::new()
    };
    let out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<MPD xmlns=\"urn:mpeg:dash:schema:mpd:2011\" type=\"{kind}\" mediaPresentationDuration=\"PT{seconds}S\" minBufferTime=\"PT2S\" profiles=\"urn:mpeg:dash:profile:isoff-on-demand:2011\">
<Period>
<AdaptationSet mimeType=\"video/mp4\" codecs=\"avc1.4d401f\">{protection}{template}
<Representation id=\"v0\" bandwidth=\"800000\" width=\"640\" height=\"360\"/>
<Representation id=\"v1\" bandwidth=\"2500000\" width=\"1280\" height=\"720\"/>
</AdaptationSet>
{audio}
</Period>
</MPD>
"
    );
    text(out, "application/dash+xml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_come_from_the_query() {
        let o = StreamOptions::parse(Some(
            "segments=3&size=10&split=1&aes=1&fail_seg=2&fail_times=4",
        ));
        assert_eq!((o.segments, o.size, o.split, o.aes), (3, 10, true, true));
        assert_eq!((o.fail_seg, o.fail_times), (Some(2), 4));
        assert_eq!(
            o.suffix(),
            "?segments=3&size=10&split=1&aes=1&fail_seg=2&fail_times=4"
        );
        assert_eq!(StreamOptions::parse(None).suffix(), "");
    }

    #[test]
    fn fragments_differ_by_track_and_number() {
        assert_ne!(fragment("v0", 0, 64), fragment("v0", 1, 64));
        assert_ne!(fragment("v0", 0, 64), fragment("v1", 0, 64));
        assert_eq!(joined("v1", "segments=2&size=8").len(), 16);
        assert_eq!(joined("audio", "segments=2&size=8&split=1").len(), 1016);
    }

    #[test]
    fn a_flat_stream_keeps_its_playlists_beside_the_master() {
        let opts = StreamOptions::parse(Some("flat=1&split=1"));
        let master = master_playlist(&opts);
        assert!(master.contains("URI=\"audio.m3u8?flat=1&split=1\""));
        assert!(master.contains("\nv1.m3u8?flat=1&split=1\n"));
        assert!(!master.contains("index.m3u8"));
        let media = media_playlist("v1", &opts, "v1/");
        assert!(media.contains("#EXT-X-MAP:URI=\"v1/init.mp4?flat=1&split=1\""));
        assert!(media.contains("\nv1/seg-0.m4s?flat=1&split=1\n"));
        // The usual layout is unchanged.
        let usual = StreamOptions::parse(Some("split=1"));
        assert!(master_playlist(&usual).contains("\nv0/index.m3u8?split=1\n"));
        assert!(media_playlist("v0", &usual, "").contains("\nseg-0.m4s?split=1\n"));
    }

    #[test]
    fn encryption_pads_to_whole_blocks() {
        assert_eq!(encrypt(&[1; 16], &VIDEO_IV).len(), 32);
        assert_eq!(iv("audio", 1)[15], 6);
    }
}
