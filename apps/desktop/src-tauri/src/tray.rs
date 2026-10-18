//! System tray icon: keeps Downpipe reachable when the window is closed, so
//! the browser extension can still hand downloads over.

use downpipe_core::Manager;
use tauri::Wry;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};

use crate::i18n::{Lang, Texts};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager as _};

const TRAY_ID: &str = "downpipe";

fn menu(app: &AppHandle, texts: &Texts) -> tauri::Result<Menu<Wry>> {
    let show = MenuItem::with_id(app, "show", texts.show, true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause_all", texts.pause_all, true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume_all", texts.resume_all, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", texts.quit, true, None::<&str>)?;
    Menu::with_items(
        app,
        &[
            &show,
            &PredefinedMenuItem::separator(app)?,
            &pause,
            &resume,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )
}

/// Rebuild the menu in another language. The handlers stay: they go by id.
pub fn set_language(app: &AppHandle, texts: &Texts) {
    if let Some(tray) = app.tray_by_id(TRAY_ID)
        && let Ok(menu) = menu(app, texts)
    {
        let _ = tray.set_menu(Some(menu));
    }
}

/// Created in English; the background loop switches it to the user's
/// language within a second, once the settings are at hand.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = menu(app, Lang::En.texts())?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Downpipe")
        .menu(&menu)
        // Left click opens the window; the menu is on right click, as
        // Windows users expect from tray icons.
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => show_main(app),
            "pause_all" => app.state::<Manager>().pause_all(),
            "resume_all" => app.state::<Manager>().resume_all(),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder.build(app)?;
    Ok(())
}

pub fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

pub fn set_tooltip(app: &AppHandle, text: &str) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(text));
    }
}
