//! Ordinary app windows (Settings). The popup panel lives in `popup.rs`.

use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder};

const SETTINGS: &str = "settings";

/// Shows the Settings window, creating it on first use. `section` selects a
/// pane ("translator", "llm", "general").
pub fn open_settings(app: &AppHandle, section: Option<&str>) {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        if let Some(section) = section {
            let _ = window.emit("settings:section", section);
        }
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    let url = match section {
        Some(section) => format!("src/settings/index.html#{section}"),
        None => "src/settings/index.html".to_string(),
    };
    let built = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(url.into()))
        .title("Hutarka Settings")
        .inner_size(900.0, 600.0)
        .min_inner_size(760.0, 480.0)
        .build();
    match built {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(e) => log::error!("cannot open Settings: {e}"),
    }
}
