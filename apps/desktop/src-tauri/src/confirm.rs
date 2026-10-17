//! The confirmation window: IDM's "Download File Info" in a small window of
//! its own, over the browser.
//!
//! A download the browser sends while Settings, App, "Ask before browser
//! downloads start" is on waits here for its name, folder and queue (and a
//! stream's or video page's quality). The main window stays as it was,
//! hidden in the tray or not. The page (`confirm.html`) asks for one
//! download after another; the window hides once none waits, which the core
//! decides, so an answer given in either window reaches both as events.
//! Video pages added in the main window are asked there instead.
//!
//! The window is built hidden the first time it is needed and kept, hidden,
//! for the next question, which then shows at once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use parking_lot::Mutex;
use tauri::{
    AppHandle, Emitter, Manager as _, PhysicalSize, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::tray;

pub const LABEL: &str = "confirm";
const PAGE: &str = "confirm.html";
/// To the page: its window's close button was clicked. That is "Download
/// later" for the download shown, as Escape is.
pub const CLOSE: &str = "downpipe://confirm-close";
/// To the page: back from Settings, read the download shown again (yt-dlp
/// or FFmpeg may be set up now).
pub const AGAIN: &str = "downpipe://confirm-again";
/// To the main window: open Settings, Media.
pub const MEDIA_SETTINGS: &str = "downpipe://media-settings";

/// IDM's dialog is as wide as this, in CSS pixels; the page asks for the
/// height its question needs (`fit`), which only the screen limits.
pub(crate) const WIDTH: f64 = 520.0;
const HEIGHT: f64 = 300.0;
const MIN_HEIGHT: f64 = 160.0;
/// Room for rounding: at 125 % display scaling a fraction of a pixel too
/// little already shows a scrollbar. Better a little taller.
const SPARE: f64 = 2.0;

/// A page that never says it drew the question is shown anyway after this,
/// so that a download cannot wait for an answer nobody sees.
const PAGE_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Phase {
    /// Nothing waits: no window yet, or a hidden one kept for later.
    #[default]
    Idle,
    /// A question waits for the page to draw it. Hidden meanwhile, so no
    /// empty or stale window flashes.
    Opening,
    Shown,
    /// Hidden while the main window's Settings are open on its behalf.
    Parked,
}

/// What happened, as far as the window cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Input {
    /// A download from the browser started waiting.
    Question,
    /// The page drew a question, or it took too long to.
    Drawn,
    /// No download from the browser waits any more.
    Settled,
    /// The page opened Settings in the main window.
    ToSettings,
    /// The main window's Settings closed, or that window went to the tray.
    BackFromSettings,
}

/// What to do to the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Act {
    Nothing,
    /// Make sure the window exists; it shows once the page drew the question.
    Open,
    /// Show it and bring it to the front.
    Show,
    /// The same, and have the page read the download again.
    ShowAgain,
    Hide,
}

fn step(phase: Phase, input: Input) -> (Phase, Act) {
    match (phase, input) {
        (Phase::Idle, Input::Question) => (Phase::Opening, Act::Open),
        // Up already: brought forward for the newcomer, which waits its turn.
        (Phase::Shown, Input::Question) => (Phase::Shown, Act::Show),
        (Phase::Opening | Phase::Shown, Input::Drawn) => (Phase::Shown, Act::Show),
        (Phase::Idle, Input::Settled) => (Phase::Idle, Act::Nothing),
        (_, Input::Settled) => (Phase::Idle, Act::Hide),
        (Phase::Opening | Phase::Shown, Input::ToSettings) => (Phase::Parked, Act::Hide),
        (Phase::Parked, Input::BackFromSettings) => (Phase::Shown, Act::ShowAgain),
        // A late call after the last answer, or a question that keeps until
        // Settings close.
        (phase, _) => (phase, Act::Nothing),
    }
}

/// Shared state, managed by Tauri.
#[derive(Debug, Default)]
pub struct ConfirmWindow {
    phase: Mutex<Phase>,
    /// The page drew a question once, so it is loaded and answers the close
    /// button itself.
    page_up: AtomicBool,
}

impl ConfirmWindow {
    fn advance(&self, input: Input) -> Act {
        let mut phase = self.phase.lock();
        let (next, act) = step(*phase, input);
        *phase = next;
        act
    }
}

fn feed(app: &AppHandle, input: Input) {
    let act = app.state::<ConfirmWindow>().advance(input);
    apply(app, act);
}

fn apply(app: &AppHandle, act: Act) {
    match act {
        Act::Nothing => {}
        Act::Open => open(app),
        Act::Show | Act::ShowAgain => {
            let Some(window) = app.get_webview_window(LABEL) else {
                // Gone (its web view failed): build it again and wait.
                *app.state::<ConfirmWindow>().phase.lock() = Phase::Opening;
                open(app);
                return;
            };
            let _ = window.unminimize();
            let _ = window.show();
            let _ = window.set_focus();
            if act == Act::ShowAgain {
                let _ = app.emit_to(LABEL, AGAIN, ());
            }
        }
        Act::Hide => {
            if let Some(window) = app.get_webview_window(LABEL) {
                let _ = window.hide();
            }
        }
    }
}

fn open(app: &AppHandle) {
    if app.get_webview_window(LABEL).is_none()
        && let Err(err) = build(app)
    {
        // Answering is still possible in the main window's list: resuming a
        // download that waits starts it as it is.
        tracing::warn!("no confirmation window: {err}");
        *app.state::<ConfirmWindow>().phase.lock() = Phase::Idle;
        tray::show_main(app);
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(PAGE_TIMEOUT).await;
        let state = app.state::<ConfirmWindow>();
        if *state.phase.lock() == Phase::Opening {
            tracing::warn!("the confirmation page did not draw in time; showing it anyway");
            feed(&app, Input::Drawn);
        }
    });
}

fn build(app: &AppHandle) -> tauri::Result<WebviewWindow> {
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App(PAGE.into()))
        .title("Confirm download - Downpipe")
        .inner_size(WIDTH, HEIGHT)
        // A dialog, sized to its question; never a page to maximize.
        .resizable(false)
        .maximizable(false)
        .center()
        // Over the browser the download came from, as IDM's dialog is.
        .always_on_top(true)
        .visible(false)
        .disable_drag_drop_handler()
        .build()
}

/// The inner size, in physical pixels, for a page that needs `height` CSS
/// pixels at `scale` physical pixels per CSS pixel, no taller than `limit`.
/// The page's `devicePixelRatio` is the scale rather than the window's,
/// since it also counts a zoom of the web view, and pixels are rounded up.
pub(crate) fn fitted(height: f64, scale: f64, limit: u32) -> PhysicalSize<u32> {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale.clamp(0.5, 8.0)
    } else {
        1.0
    };
    let height = if height.is_finite() {
        height.max(MIN_HEIGHT)
    } else {
        HEIGHT
    };
    let wanted = ((height + SPARE) * scale).ceil() as u32;
    PhysicalSize::new((WIDTH * scale).round() as u32, wanted.min(limit.max(1)))
}

/// The tallest inner height that keeps the window on its screen, above the
/// taskbar.
fn room(window: &WebviewWindow) -> u32 {
    let frame = match (window.outer_size(), window.inner_size()) {
        (Ok(outer), Ok(inner)) => outer.height.saturating_sub(inner.height),
        _ => 0,
    };
    window
        .current_monitor()
        .ok()
        .flatten()
        .map_or(u32::MAX, |m| {
            m.work_area().size.height.saturating_sub(frame)
        })
}

/// A dialog window's page (this one or a download's progress window)
/// needs `height` CSS pixels at `scale` (its `devicePixelRatio`). The
/// confirmation window, about to show, is centred again for it, as IDM's
/// dialog opens in the middle.
pub fn fit(window: &WebviewWindow, height: f64, scale: f64) {
    let _ = window.set_size(fitted(height, scale, room(window)));
    if window.label() == LABEL
        && *window.app_handle().state::<ConfirmWindow>().phase.lock() == Phase::Opening
    {
        let _ = window.center();
    }
}

/// A download from the browser started waiting for its answer.
pub fn question(app: &AppHandle) {
    feed(app, Input::Question);
}

/// No download from the browser waits for an answer any more.
pub fn settled(app: &AppHandle) {
    feed(app, Input::Settled);
}

/// The page drew a question and can be shown.
pub fn drawn(app: &AppHandle) {
    app.state::<ConfirmWindow>()
        .page_up
        .store(true, Ordering::Relaxed);
    feed(app, Input::Drawn);
}

/// The page's Settings links: the main window opens on Settings, Media,
/// and the question waits, hidden, until those close.
pub fn to_settings(app: &AppHandle) {
    feed(app, Input::ToSettings);
    tray::show_main(app);
    let _ = app.emit_to("main", MEDIA_SETTINGS, ());
}

/// The main window's Settings closed, or the main window went to the tray
/// with them still open.
pub fn back_from_settings(app: &AppHandle) {
    feed(app, Input::BackFromSettings);
}

/// The window's close button. The page answers "Download later" for the
/// download shown, and the window hides once none waits.
pub fn close_requested(app: &AppHandle) {
    let state = app.state::<ConfirmWindow>();
    let shown = *state.phase.lock() == Phase::Shown;
    if shown && state.page_up.load(Ordering::Relaxed) {
        let _ = app.emit_to(LABEL, CLOSE, ());
    } else {
        // A page that never loaded cannot answer; the next download from the
        // browser tries again.
        *state.phase.lock() = Phase::Idle;
        apply(app, Act::Hide);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(inputs: &[Input]) -> (Phase, Vec<Act>) {
        let mut phase = Phase::Idle;
        let mut acts = Vec::new();
        for &input in inputs {
            let (next, act) = step(phase, input);
            phase = next;
            acts.push(act);
        }
        (phase, acts)
    }

    #[test]
    fn the_window_takes_the_height_asked_within_bounds() {
        let free = u32::MAX;
        assert_eq!(fitted(272.4, 1.0, free), PhysicalSize::new(520, 275));
        // 125 % and 150 % display scaling: rounded up, never a pixel short.
        assert_eq!(fitted(272.4, 1.25, free), PhysicalSize::new(650, 343));
        assert_eq!(fitted(272.4, 1.5, free), PhysicalSize::new(780, 412));
        assert_eq!(fitted(40.0, 1.0, free).height, 162, "never a sliver");
        assert_eq!(
            fitted(4000.0, 1.5, 1000).height,
            1000,
            "only the screen limits it"
        );
        assert_eq!(
            fitted(f64::NAN, f64::NAN, free),
            PhysicalSize::new(520, 302)
        );
    }

    #[test]
    fn a_question_opens_the_window_once_the_page_drew_it() {
        let (phase, acts) = run(&[Input::Question, Input::Drawn]);
        assert_eq!(phase, Phase::Shown);
        assert_eq!(acts, [Act::Open, Act::Show]);
    }

    #[test]
    fn more_questions_bring_it_forward_and_the_last_answer_hides_it() {
        let (phase, acts) = run(&[
            Input::Question,
            Input::Question,
            Input::Drawn,
            Input::Question,
            Input::Drawn,
            Input::Settled,
        ]);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(
            acts,
            [
                Act::Open,
                Act::Nothing,
                Act::Show,
                Act::Show,
                Act::Show,
                Act::Hide
            ]
        );
    }

    #[test]
    fn nothing_happens_while_nothing_waits() {
        let (phase, acts) = run(&[Input::Settled, Input::Drawn, Input::BackFromSettings]);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(acts, [Act::Nothing, Act::Nothing, Act::Nothing]);
    }

    #[test]
    fn settings_park_the_question_until_they_close() {
        let (phase, acts) = run(&[
            Input::Question,
            Input::Drawn,
            Input::ToSettings,
            // Neither a newcomer nor the page shows it over the Settings.
            Input::Question,
            Input::Drawn,
            Input::BackFromSettings,
        ]);
        assert_eq!(phase, Phase::Shown);
        assert_eq!(
            acts,
            [
                Act::Open,
                Act::Show,
                Act::Hide,
                Act::Nothing,
                Act::Nothing,
                Act::ShowAgain
            ]
        );
    }

    #[test]
    fn a_parked_question_resumed_from_the_list_stays_hidden() {
        let (phase, acts) = run(&[
            Input::Question,
            Input::Drawn,
            Input::ToSettings,
            Input::Settled,
            Input::BackFromSettings,
        ]);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(
            acts,
            [Act::Open, Act::Show, Act::Hide, Act::Hide, Act::Nothing]
        );
    }

    #[test]
    fn a_question_answered_before_the_page_drew_it_never_shows() {
        let (phase, acts) = run(&[Input::Question, Input::Settled, Input::Drawn]);
        assert_eq!(phase, Phase::Idle);
        assert_eq!(acts, [Act::Open, Act::Hide, Act::Nothing]);
    }
}
