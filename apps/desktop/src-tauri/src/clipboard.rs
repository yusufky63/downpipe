//! Clipboard watching: a copied download link brings up the Add dialog, as
//! in IDM. Only http(s) links whose file type belongs to a category, that
//! are video pages the site provider takes (YouTube), or magnet links and
//! `.torrent` links (BitTorrent) are offered, and never
//! one that is already in the list, so "Copy link" in Downpipe's own menu
//! does not offer the link back.

use std::time::{Duration, Instant};

use downpipe_core::Manager;
use downpipe_core::providers::pages;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::tray;

/// Sent to the web view with the link to offer.
pub const EVENT: &str = "downpipe://clipboard";

const EVERY: Duration = Duration::from_millis(750);
/// Some programs write the clipboard several times for one copy; one offer
/// is enough.
const SAME_OFFER: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Serialize)]
struct Offer {
    url: String,
}

/// Start the watcher thread. It sleeps while the setting is off.
pub fn spawn(app: AppHandle, manager: Manager) {
    let started = std::thread::Builder::new()
        .name("clipboard".into())
        .spawn(move || watch(&app, &manager));
    if let Err(err) = started {
        tracing::warn!("clipboard watching is off: {err}");
    }
}

fn watch(app: &AppHandle, manager: &Manager) {
    let mut clipboard: Option<arboard::Clipboard> = None;
    // Unset until the first look after watching is switched on, so what was
    // copied before that is never offered.
    let mut primed = false;
    let mut last_stamp: Option<u32> = None;
    let mut last_text: Option<String> = None;
    let mut last_offer: Option<(String, Instant)> = None;
    loop {
        std::thread::sleep(EVERY);
        let settings = manager.settings();
        if !settings.watch_clipboard {
            primed = false;
            clipboard = None;
            continue;
        }
        let stamp = sequence();
        if primed && stamp.is_some() && stamp == last_stamp {
            continue;
        }
        let text = match read(&mut clipboard) {
            Ok(text) => text,
            // Another program holds the clipboard open; look again next time.
            Err(()) => continue,
        };
        let first = !primed;
        primed = true;
        last_stamp = stamp;
        // Without a change counter the same text copied twice looks like
        // no change at all.
        let unchanged = stamp.is_none() && text == last_text;
        last_text.clone_from(&text);
        if first || unchanged {
            continue;
        }
        let Some(url) = text
            .as_deref()
            .and_then(|t| offer(t, &settings.watched_extensions()))
        else {
            continue;
        };
        if last_offer
            .as_ref()
            .is_some_and(|(prev, at)| *prev == url && at.elapsed() < SAME_OFFER)
        {
            continue;
        }
        let key = list_key(&url);
        let lower = url.to_ascii_lowercase();
        if manager.list().iter().any(|t| {
            t.record.url == key
                || t.record.torrent.as_ref().is_some_and(|torrent| {
                    !torrent.info_hash.is_empty()
                        && lower.contains(&format!("urn:btih:{}", torrent.info_hash))
                })
        }) {
            continue;
        }
        last_offer = Some((url.clone(), Instant::now()));
        tray::show_main(app);
        let _ = app.emit(EVENT, Offer { url });
    }
}

/// Text on the clipboard, `Ok(None)` when it holds something else, `Err`
/// when it could not be read this time.
fn read(clipboard: &mut Option<arboard::Clipboard>) -> Result<Option<String>, ()> {
    if clipboard.is_none() {
        *clipboard = arboard::Clipboard::new().ok();
    }
    let Some(board) = clipboard.as_mut() else {
        return Err(());
    };
    match board.get_text() {
        Ok(text) => Ok(Some(text)),
        Err(arboard::Error::ContentNotAvailable | arboard::Error::ConversionFailure) => Ok(None),
        Err(arboard::Error::ClipboardOccupied) => Err(()),
        Err(_) => {
            // Rebuilt next time: on Linux the connection to the display
            // server may have gone away.
            *clipboard = None;
            Err(())
        }
    }
}

/// Windows counts every clipboard change, so the text is read only when it
/// changed.
#[cfg(windows)]
fn sequence() -> Option<u32> {
    // SAFETY: takes no arguments and only reads a counter.
    Some(unsafe { windows_sys::Win32::System::DataExchange::GetClipboardSequenceNumber() })
}

#[cfg(not(windows))]
fn sequence() -> Option<u32> {
    None
}

/// The link to offer for copied text: one http(s) URL, nothing else, whose
/// file name ends in one of `extensions`, or a video page, which has no file
/// name to go by. The Add dialog sends a video page on to the quality
/// chooser.
fn offer(text: &str, extensions: &[String]) -> Option<String> {
    let text = text.trim();
    if text.is_empty() || text.len() > 4096 || text.chars().any(char::is_whitespace) {
        return None;
    }
    if downpipe_core::TORRENT_SUPPORT
        && text
            .get(..7)
            .is_some_and(|s| s.eq_ignore_ascii_case("magnet:"))
    {
        return text
            .to_ascii_lowercase()
            .contains("xt=urn:btih:")
            .then(|| text.to_string());
    }
    let url = url::Url::parse(text).ok()?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return None;
    }
    // Video pages, playlists and channels: the Add dialog takes them on to
    // the quality chooser or the list of videos; torrents to their files.
    if pages::recognize(&url).is_some()
        || pages::recognize_list(&url).is_some()
        || downpipe_core::TORRENT_SUPPORT && downpipe_core::is_torrent_link(text)
    {
        return Some(text.to_string());
    }
    let name = url.path_segments()?.next_back()?;
    let (_, ext) = name.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    extensions.contains(&ext).then(|| text.to_string())
}

/// The link as the list keeps it: without credentials, and a video page as
/// its one link per video, so a share link of a video already in the list
/// is not offered again.
fn list_key(link: &str) -> String {
    match url::Url::parse(link) {
        Ok(url) => {
            let url = pages::recognize(&url).map_or(url, |page| page.url);
            downpipe_core::http::redact_url(&url)
        }
        Err(_) => link.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exts() -> Vec<String> {
        ["iso", "zip", "mp4"].map(String::from).to_vec()
    }

    #[test]
    fn offers_single_links_to_known_file_types() {
        assert_eq!(
            offer("  https://example.org/a/Disk.ISO?x=1#top \n", &exts()).as_deref(),
            Some("https://example.org/a/Disk.ISO?x=1#top")
        );
        assert!(offer("http://example.org/a.zip", &exts()).is_some());
    }

    #[test]
    fn ignores_everything_else() {
        for text in [
            "",
            "hello world",
            "https://example.org/",
            "https://example.org/page.html",
            "https://example.org/a.zip https://example.org/b.zip",
            "ftp://example.org/a.zip",
            "file:///C:/a.zip",
            "C:\\Downloads\\a.zip",
            "https://example.org/zip",
            "https://example.org/a.zip/",
        ] {
            assert_eq!(offer(text, &exts()), None, "{text:?}");
        }
        let long = format!("https://example.org/{}.zip", "a".repeat(5000));
        assert_eq!(offer(&long, &exts()), None);
    }

    #[test]
    fn offers_video_pages_whatever_the_categories() {
        for text in [
            "https://www.youtube.com/watch?v=jNQXAC9IVRw",
            "https://youtu.be/jNQXAC9IVRw?si=abc",
            " https://www.youtube.com/shorts/aqz-KE-bpKQ\n",
            "https://www.youtube.com/playlist?list=PLbpi6ZahtOH6Blw3RGYpWkSByi_T7Rygb",
            "https://www.youtube.com/@jawed",
        ] {
            assert_eq!(offer(text, &[]).as_deref(), Some(text.trim()), "{text:?}");
        }
        for text in [
            "https://www.youtube.com/",
            // Watch later needs the browser's sign-in.
            "https://www.youtube.com/playlist?list=WL",
            "https://www.youtube.com/watch?v=short",
            "https://youtu.be/jNQXAC9IVRw https://youtu.be/aqz-KE-bpKQ",
        ] {
            assert_eq!(offer(text, &exts()), None, "{text:?}");
        }
    }

    #[test]
    fn offers_torrents() {
        let magnet = "magnet:?xt=urn:btih:c9e15763f722f23e98a29decdfae341b98d53056&dn=a";
        assert_eq!(offer(&format!(" {magnet} "), &[]).as_deref(), Some(magnet));
        assert_eq!(offer("magnet:?dn=no-hash", &[]), None);
        assert!(offer("https://example.org/ubuntu.iso.torrent", &[]).is_some());
    }

    #[test]
    fn keys_match_the_list() {
        // A video is listed under its canonical page, so any link to it is
        // recognized as already there.
        assert_eq!(
            list_key("https://youtu.be/jNQXAC9IVRw?si=abc&t=3"),
            "https://www.youtube.com/watch?v=jNQXAC9IVRw"
        );
        assert_eq!(
            list_key("https://user:pw@example.org/a.zip"),
            "https://example.org/a.zip"
        );
    }
}
