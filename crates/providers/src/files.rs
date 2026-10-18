//! The files a site download leaves in its folder.
//!
//! yt-dlp names everything after the output template's stem: the finished
//! `Clip.mp4`, and while it works `Clip.f137.mp4.part` (a format being
//! downloaded), `Clip.f137.mp4` (a format waiting to be merged),
//! `Clip.temp.mp4` (the merge in progress) and `Clip.f137.mp4.ytdl` or
//! `.part-Frag3` for formats downloaded in fragments. Downpipe picks the
//! stem once, before the first start, so that a resume finds these again
//! and a second download of the same video does not land on the first.

use std::path::{Path, PathBuf};

/// Extensions a finished site download can have.
const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "webm", "m4a", "mp3", "opus", "ogg", "mov", "flv", "avi", "aac", "wav", "flac",
    "3gp", "ts", "m4v",
];

/// Subtitle files yt-dlp writes next to a video: `Clip.en.vtt`.
const SUBTITLE_EXTENSIONS: &[&str] = &[
    "vtt", "srt", "ass", "ssa", "ttml", "srv1", "srv2", "srv3", "json3", "lrc",
];

/// Longest stem picked from a title, in characters; leaves room for
/// `(12).f137.mp4.part` within the usual 255.
const MAX_STEM: usize = 180;

/// A stored file name without its media extension: the stem yt-dlp writes
/// under. `Clip.mp4` gives `Clip`, `Version 1.2` stays as it is.
pub fn stem_of(file_name: &str) -> String {
    match file_name.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty()
                && MEDIA_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) =>
        {
            stem.to_string()
        }
        _ => file_name.to_string(),
    }
}

/// A stem made from a page title: a safe file name, shortened.
pub fn stem_from_title(title: &str) -> String {
    if title.trim().is_empty() {
        return "video".to_string();
    }
    let name = downpipe_http::filename::sanitize(title);
    let short: String = name.chars().take(MAX_STEM).collect();
    let short = short.trim_end_matches(['.', ' ']).to_string();
    if short.is_empty() {
        "video".to_string()
    } else {
        short
    }
}

/// What the rest of a name after `stem.` says about it.
fn kind_of(rest: &str) -> Option<Kind> {
    let lower = rest.to_ascii_lowercase();
    if lower.ends_with(".part") || lower.contains(".part-frag") || lower.ends_with(".ytdl") {
        return Some(Kind::Partial);
    }
    if lower.starts_with("temp.") {
        return Some(Kind::Partial);
    }
    // f<format id>.<ext>: a format downloaded and waiting to be merged.
    if let Some(after_f) = lower.strip_prefix('f')
        && let Some((id, ext)) = after_f.rsplit_once('.')
        && !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        && MEDIA_EXTENSIONS.contains(&ext)
    {
        return Some(Kind::Partial);
    }
    // <language>.<subtitle extension>
    if let Some((lang, ext)) = lower.rsplit_once('.')
        && SUBTITLE_EXTENSIONS.contains(&ext)
        && crate::args::valid_sub_lang(lang)
    {
        return Some(Kind::Subtitle);
    }
    MEDIA_EXTENSIONS
        .contains(&lower.as_str())
        .then_some(Kind::Finished)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Partial,
    /// Written before the video; part of the result once it is there.
    Subtitle,
    Finished,
}

fn entries(dir: &Path, stem: &str) -> Vec<(PathBuf, Kind)> {
    let prefix = format!("{stem}.");
    let Ok(read) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    read.filter_map(Result::ok)
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let rest = name.strip_prefix(&prefix)?;
            kind_of(rest).map(|kind| (e.path(), kind))
        })
        .collect()
}

/// Work files of a download under `stem` in `dir`.
pub fn partials(dir: &Path, stem: &str) -> Vec<PathBuf> {
    entries(dir, stem)
        .into_iter()
        .filter(|(_, k)| *k == Kind::Partial)
        .map(|(p, _)| p)
        .collect()
}

/// Delete the work files of a canceled or removed download, and its
/// subtitles while the video is not finished. The finished file, if there
/// is one, is left alone with its subtitles.
pub fn remove_partials(dir: &Path, stem: &str) {
    let found = entries(dir, stem);
    let finished = found.iter().any(|(_, k)| *k == Kind::Finished);
    let doomed = found
        .into_iter()
        .filter(|(_, k)| *k == Kind::Partial || (*k == Kind::Subtitle && !finished))
        .map(|(p, _)| p);
    for path in doomed {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::debug!("could not delete {}: {e}", path.display());
        }
    }
}

/// `stem` itself when a download under it is under way (work files exist)
/// or nothing uses it yet; otherwise the first free `stem (n)`.
pub fn unique_stem(dir: &Path, stem: &str) -> String {
    // Subtitles alone mean a download under way: they come first.
    let free = |s: &str| {
        let found = entries(dir, s);
        found.iter().any(|(_, k)| *k == Kind::Partial)
            || !found.iter().any(|(_, k)| *k == Kind::Finished)
    };
    if free(stem) {
        return stem.to_string();
    }
    (1..1000)
        .map(|n| format!("{stem} ({n})"))
        .find(|s| entries(dir, s).is_empty())
        .unwrap_or_else(|| format!("{stem} ({})", std::process::id()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stems() {
        assert_eq!(stem_of("Clip.mp4"), "Clip");
        assert_eq!(stem_of("Clip.MKV"), "Clip");
        assert_eq!(stem_of("Version 1.2"), "Version 1.2");
        assert_eq!(stem_of(".mp4"), ".mp4");
        assert_eq!(
            stem_from_title("a/b: c?"),
            downpipe_http::filename::sanitize("a/b: c?")
        );
        assert_eq!(stem_from_title("   "), "video");
        assert_eq!(stem_from_title(&"x".repeat(400)).chars().count(), MAX_STEM);
    }

    #[test]
    fn work_files_and_finished_files() {
        let dir = tempfile::tempdir().unwrap();
        let touch = |name: &str| std::fs::write(dir.path().join(name), b"x").unwrap();
        for name in [
            "Clip.f137.mp4.part",
            "Clip.f140-drc.m4a",
            "Clip.temp.mp4",
            "Clip.f399.mp4.part-Frag3",
            "Clip.f399.mp4.ytdl",
            "Clip.mp4.part",
            "Clip.mp4",
            "Clip.description",
            "Clip (1).f137.mp4.part",
            "Clips.mp4.part",
        ] {
            touch(name);
        }
        let mut found: Vec<String> = partials(dir.path(), "Clip")
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        found.sort();
        assert_eq!(
            found,
            [
                "Clip.f137.mp4.part",
                "Clip.f140-drc.m4a",
                "Clip.f399.mp4.part-Frag3",
                "Clip.f399.mp4.ytdl",
                "Clip.mp4.part",
                "Clip.temp.mp4"
            ]
        );
        remove_partials(dir.path(), "Clip");
        assert!(partials(dir.path(), "Clip").is_empty());
        // The finished file and files of other names stay.
        for kept in [
            "Clip.mp4",
            "Clip.description",
            "Clip (1).f137.mp4.part",
            "Clips.mp4.part",
        ] {
            assert!(dir.path().join(kept).exists(), "{kept}");
        }
    }

    #[test]
    fn picks_a_free_stem() {
        let dir = tempfile::tempdir().unwrap();
        let touch = |name: &str| std::fs::write(dir.path().join(name), b"x").unwrap();
        assert_eq!(unique_stem(dir.path(), "Clip"), "Clip");
        // A download under way keeps its stem.
        touch("Clip.f137.mp4.part");
        assert_eq!(unique_stem(dir.path(), "Clip"), "Clip");
        std::fs::remove_file(dir.path().join("Clip.f137.mp4.part")).unwrap();
        // A finished one does not get overwritten or taken for done.
        touch("Clip.mkv");
        assert_eq!(unique_stem(dir.path(), "Clip"), "Clip (1)");
        touch("Clip (1).m4a");
        assert_eq!(unique_stem(dir.path(), "Clip"), "Clip (2)");
        // Subtitles written before a paused video keep the stem.
        touch("Talk.en.vtt");
        assert_eq!(unique_stem(dir.path(), "Talk"), "Talk");
    }

    #[test]
    fn subtitles_go_with_an_unfinished_video_only() {
        let dir = tempfile::tempdir().unwrap();
        let touch = |name: &str| std::fs::write(dir.path().join(name), b"x").unwrap();
        touch("Clip.en.vtt");
        touch("Clip.f137.mp4.part");
        touch("Clip.notes.txt");
        remove_partials(dir.path(), "Clip");
        assert!(!dir.path().join("Clip.en.vtt").exists());
        assert!(dir.path().join("Clip.notes.txt").exists());
        touch("Done.mp4");
        touch("Done.tr.srt");
        remove_partials(dir.path(), "Done");
        assert!(dir.path().join("Done.tr.srt").exists());
    }
}
