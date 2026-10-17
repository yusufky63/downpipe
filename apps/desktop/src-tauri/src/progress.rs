//! Progress windows: IDM's download dialog. A download started from the
//! confirmation window gets a small window of its own that follows it:
//! status, size, speed, time left and the segment bar, with Pause or
//! Resume, Cancel and Hide; once it finishes, Open, Open folder and Close.
//! Hiding closes the window only; the download goes on in the list.
//!
//! One window per download, labelled `progress-<id>`, with the same page as
//! the confirmation window (`confirm.html` draws a download's progress for
//! such a label), which also keeps the window's title in step. Unlike the
//! confirmation window it is closed, not kept, when hidden, so a day of
//! downloads does not leave a web view behind for each.

use std::time::Duration;

use downpipe_core::Manager;
use tauri::{
    AppHandle, Manager as _, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::confirm;

pub const PREFIX: &str = "progress-";
const PAGE: &str = "confirm.html";
const HEIGHT: f64 = 240.0;
/// Each further window opens this much lower and to the right, so none
/// hides another completely; after `CASCADE_STEPS` they start over.
const CASCADE: f64 = 28.0;
const CASCADE_STEPS: usize = 8;
/// A page that never says it is drawn is shown anyway after this.
const PAGE_TIMEOUT: Duration = Duration::from_secs(3);
/// Longer titles are cut; the taskbar shows far less anyway.
const TITLE_MAX: usize = 200;

/// The window label for a download id. Ids are UUIDs; anything else is
/// refused rather than put into a label.
fn label_for(id: &str) -> Option<String> {
    let plain = !id.is_empty()
        && id.len() <= 64
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    plain.then(|| format!("{PREFIX}{id}"))
}

/// The download a progress window follows.
pub fn id_of(label: &str) -> Option<&str> {
    label.strip_prefix(PREFIX).filter(|id| !id.is_empty())
}

/// Where the next window opens, in physical pixels: at `origin` (the
/// confirmation window, so the question seems to turn into its progress),
/// `index` steps further down at `scale`.
fn cascade(origin: PhysicalPosition<i32>, scale: f64, index: usize) -> PhysicalPosition<i32> {
    let step = (CASCADE * scale * (index % CASCADE_STEPS) as f64).round() as i32;
    PhysicalPosition::new(origin.x + step, origin.y + step)
}

/// The first step, from `first` on, where no progress window stands yet;
/// when every step is taken, the next one in turn.
fn free_step(
    taken: &[PhysicalPosition<i32>],
    origin: PhysicalPosition<i32>,
    scale: f64,
    first: usize,
) -> usize {
    (first..first + CASCADE_STEPS)
        .find(|&i| !taken.contains(&cascade(origin, scale, i)))
        .unwrap_or(first + taken.len())
}

/// A title from the page, on one line and of a sane length.
fn clean_title(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(TITLE_MAX)
        .collect::<String>()
        .trim()
        .to_string()
}

/// Follow a download in its own window, or bring its window forward.
pub fn open(app: &AppHandle, id: &str) {
    let Some(label) = label_for(id) else {
        return;
    };
    if let Some(window) = app.get_webview_window(&label) {
        let _ = window.unminimize();
        let _ = window.show();
        confirm::bring_forward(&window);
        return;
    }
    let manager = app.state::<Manager>();
    if manager.get(id).is_none() {
        return;
    }
    let taken: Vec<PhysicalPosition<i32>> = app
        .webview_windows()
        .iter()
        .filter(|(l, _)| l.starts_with(PREFIX))
        .filter_map(|(_, w)| w.outer_position().ok())
        .collect();
    // With more questions waiting, the confirmation window stays where it
    // is, over this one: one step down keeps this one in sight.
    let more = manager
        .list()
        .iter()
        .any(|t| t.live.awaiting_confirmation && !t.live.from_app);
    // Physical pixels from window to window: a logical position would be
    // converted with whichever screen's scale the new window starts on.
    let origin = app
        .get_webview_window(confirm::LABEL)
        .and_then(|w| Some((w.outer_position().ok()?, w.scale_factor().ok()?)));
    let builder = WebviewWindowBuilder::new(app, &label, WebviewUrl::App(PAGE.into()))
        // The page names it after the download as soon as it loads.
        .title("Downpipe")
        .inner_size(confirm::WIDTH, HEIGHT)
        .resizable(false)
        .maximizable(false)
        .visible(false)
        .center()
        .disable_drag_drop_handler();
    let window = match builder.build() {
        Ok(window) => window,
        Err(err) => {
            tracing::warn!("no progress window: {err}");
            return;
        }
    };
    if let Some((at, scale)) = origin {
        let index = free_step(&taken, at, scale, usize::from(more));
        let _ = window.set_position(cascade(at, scale, index));
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PAGE_TIMEOUT).await;
        if let Some(window) = app.get_webview_window(&label)
            && !window.is_visible().unwrap_or(true)
        {
            tracing::warn!("the progress page did not draw in time; showing it anyway");
            let _ = window.show();
        }
    });
}

/// The page drew the download and the window has its height: show it. It
/// follows a click on Start download, so it may take the focus; like every
/// Downpipe window it then keeps the normal order.
pub fn drawn(window: &WebviewWindow) {
    if id_of(window.label()).is_some() && !window.is_visible().unwrap_or(false) {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// The page's title for its window: in the taskbar it is all that is seen
/// of a download running behind other windows, so it carries the progress.
pub fn set_title(window: &WebviewWindow, title: &str) {
    if id_of(window.label()).is_some() {
        let _ = window.set_title(&clean_title(title));
    }
}

/// Hide, Close and Cancel: the window goes, the download stays in the list.
pub fn close(window: &WebviewWindow) {
    if id_of(window.label()).is_some() {
        let _ = window.destroy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_carry_plain_ids_only() {
        let id = "68b6ffff-1a2b-4c3d-9e8f-0123456789ab";
        let label = label_for(id).unwrap();
        assert_eq!(label, format!("progress-{id}"));
        assert_eq!(id_of(&label), Some(id));
        assert_eq!(label_for("../confirm"), None);
        assert_eq!(label_for(""), None);
        assert_eq!(id_of("confirm"), None);
        assert_eq!(id_of("progress-"), None);
    }

    #[test]
    fn windows_cascade_from_the_question() {
        let at = PhysicalPosition::new(100, 50);
        assert_eq!(cascade(at, 1.0, 0), at);
        assert_eq!(cascade(at, 1.0, 2), PhysicalPosition::new(156, 106));
        assert_eq!(
            cascade(at, 1.5, 1),
            PhysicalPosition::new(142, 92),
            "steps scale"
        );
        assert_eq!(cascade(at, 1.0, CASCADE_STEPS), at, "starts over");
    }

    #[test]
    fn a_new_window_takes_the_first_free_step() {
        let at = PhysicalPosition::new(100, 50);
        assert_eq!(free_step(&[], at, 1.0, 0), 0);
        assert_eq!(
            free_step(&[], at, 1.0, 1),
            1,
            "a question still waits there"
        );
        // The second came while a question waited (step 2); the third
        // must not open on it.
        let taken = [cascade(at, 1.0, 0), cascade(at, 1.0, 2)];
        assert_eq!(free_step(&taken, at, 1.0, 0), 1);
        assert_eq!(free_step(&taken[..1], at, 1.0, 0), 1);
        // A window the user moved away frees its step.
        assert_eq!(free_step(&[PhysicalPosition::new(900, 900)], at, 1.0, 0), 0);
    }

    #[test]
    fn titles_stay_on_one_line() {
        assert_eq!(
            clean_title("45% a.zip\n - Downpipe"),
            "45% a.zip  - Downpipe"
        );
        assert_eq!(clean_title(&"x".repeat(500)).len(), TITLE_MAX);
    }
}
