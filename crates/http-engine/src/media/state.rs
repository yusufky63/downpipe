//! The work folder of a stream download: `<final name>.part/`.
//!
//! Every finished fragment is its own file, written as `.tmp`, synced and
//! renamed, so a file named `t0-000042.frag` is always complete: that is the
//! whole resume record. `media.json` says which stream and which tracks the
//! fragments belong to; a resume whose freshly fetched manifest lists other
//! fragments starts over instead of mixing two streams.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::resolve::{Plan, Track, TrackRole};
use super::{Container, MediaKind, MediaSelection};
use crate::DownloadError;

const STATE_FILE: &str = "media.json";
const STATE_VERSION: u32 = 1;

/// `<final>.part`: the same name a file download's partial data uses, so
/// unique names and cleanup treat both alike.
pub fn work_dir(final_path: &Path) -> PathBuf {
    crate::meta::part_path(final_path)
}

/// Delete a stream download's work folder, if it is one: only a folder
/// with Downpipe's state file in it is removed.
pub fn remove_work_dir(final_path: &Path) {
    let dir = work_dir(final_path);
    if dir.join(STATE_FILE).is_file() {
        let _ = fs::remove_dir_all(dir);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkState {
    pub version: u32,
    /// The manifest URL, credentials removed.
    pub url: String,
    pub kind: MediaKind,
    pub quality: String,
    pub selection: MediaSelection,
    pub tracks: Vec<TrackState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TrackState {
    pub role: TrackRole,
    pub container: Container,
    pub fragments: usize,
    /// Digest of the fragment list (see [`fingerprint`]).
    pub fingerprint: String,
    /// The fragments are joined into the track file and deleted.
    #[serde(default)]
    pub joined: bool,
}

impl TrackState {
    fn same_fragments(&self, other: &Self) -> bool {
        self.role == other.role
            && self.container == other.container
            && self.fragments == other.fragments
            && self.fingerprint == other.fingerprint
    }
}

impl WorkState {
    pub fn new(url: String, kind: MediaKind, plan: &Plan) -> Self {
        Self {
            version: STATE_VERSION,
            url,
            kind,
            quality: plan.quality.clone(),
            selection: plan.selection.clone(),
            tracks: plan
                .tracks
                .iter()
                .map(|t| TrackState {
                    role: t.role,
                    container: t.container,
                    fragments: t.fragment_count(),
                    fingerprint: fingerprint(t),
                    joined: false,
                })
                .collect(),
        }
    }

    /// The fragments on disk belong to this plan. The manifest URL is not
    /// compared: the folder was found by it, or the user gave the download
    /// a new link, and the fragment lists decide either way.
    pub fn same_stream(&self, other: &Self) -> bool {
        self.kind == other.kind
            && self.tracks.len() == other.tracks.len()
            && self
                .tracks
                .iter()
                .zip(&other.tracks)
                .all(|(a, b)| a.same_fragments(b))
    }
}

/// Identifies a track's fragments by path, byte range and encryption, but
/// not by query string: signed URLs get new tokens on every manifest fetch
/// while naming the same bytes.
pub(crate) fn fingerprint(track: &Track) -> String {
    let mut hash = Sha256::new();
    for segment in track.fragments() {
        hash.update(segment.url.host_str().unwrap_or("").as_bytes());
        hash.update(segment.url.path().as_bytes());
        if let Some(range) = segment.range {
            hash.update(range.offset.to_le_bytes());
            hash.update(range.length.to_le_bytes());
        }
        hash.update([u8::from(segment.key.is_some())]);
        hash.update(b"\n");
    }
    hex::encode(&hash.finalize()[..16])
}

pub(crate) fn fragment_path(work: &Path, track: usize, index: usize) -> PathBuf {
    work.join(format!("t{track}-{index:06}.frag"))
}

pub(crate) fn temp_path(work: &Path, track: usize, index: usize) -> PathBuf {
    work.join(format!("t{track}-{index:06}.tmp"))
}

/// The joined track, before merging or moving into place.
pub(crate) fn track_path(work: &Path, track: usize, container: Container) -> PathBuf {
    work.join(format!("track{track}.{}", container.extension()))
}

pub(crate) fn load(work: &Path) -> Option<WorkState> {
    let raw = fs::read(work.join(STATE_FILE)).ok()?;
    let state: WorkState = serde_json::from_slice(&raw).ok()?;
    (state.version == STATE_VERSION).then_some(state)
}

/// Write-then-rename, like the meta file of a single download.
pub(crate) fn save(work: &Path, state: &WorkState) -> Result<(), DownloadError> {
    let path = work.join(STATE_FILE);
    let tmp = work.join(format!("{STATE_FILE}.tmp"));
    let json = serde_json::to_vec_pretty(state).map_err(std::io::Error::other)?;
    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(&json)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// Empty the folder for a fresh start, keeping the folder itself.
pub(crate) fn clear(work: &Path) -> Result<(), DownloadError> {
    for entry in fs::read_dir(work)? {
        let path = entry?.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)?;
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::Segment;
    use url::Url;

    fn track(urls: &[&str]) -> Track {
        Track {
            role: TrackRole::Muxed,
            init: None,
            segments: urls
                .iter()
                .map(|u| Segment::new(Url::parse(u).unwrap()))
                .collect(),
            container: Container::Ts,
            estimated_bytes: None,
        }
    }

    #[test]
    fn fingerprints_ignore_tokens_but_not_paths() {
        let a = fingerprint(&track(&[
            "https://a.org/s/1.ts?token=x",
            "https://a.org/s/2.ts?token=x",
        ]));
        let b = fingerprint(&track(&[
            "https://a.org/s/1.ts?token=y",
            "https://a.org/s/2.ts?token=y",
        ]));
        let c = fingerprint(&track(&["https://a.org/s/1.ts", "https://a.org/s/3.ts"]));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn state_round_trips_and_only_state_folders_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let final_path = dir.path().join("clip.mp4");
        let work = work_dir(&final_path);
        fs::create_dir_all(&work).unwrap();
        let plan = Plan {
            tracks: vec![track(&["https://a.org/1.ts"])],
            quality: "720p".into(),
            selection: MediaSelection::default(),
        };
        let state = WorkState::new("https://a.org/m.m3u8".into(), MediaKind::Hls, &plan);
        save(&work, &state).unwrap();
        assert_eq!(load(&work), Some(state.clone()));
        assert!(state.same_stream(&load(&work).unwrap()));

        let other = dir.path().join("other.mp4");
        fs::create_dir_all(work_dir(&other)).unwrap();
        remove_work_dir(&other);
        assert!(work_dir(&other).exists(), "not a stream download's folder");
        remove_work_dir(&final_path);
        assert!(!work.exists());
    }
}
