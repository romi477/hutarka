//! Commands of the Settings window.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::lookup::{AppState, save_settings};
use crate::settings::{PromptId, PromptSpec, SettingsView, default_prompt};
use crate::translator::Lang;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    SettingsView::from(&app.state::<AppState>().settings())
}

#[tauri::command]
pub fn set_prompt(app: AppHandle, id: PromptId, spec: PromptSpec) {
    save_settings(&app, |s| s.prompts.set(id, spec));
}

#[tauri::command]
pub fn reset_prompt(app: AppHandle, id: PromptId) -> PromptSpec {
    let spec = default_prompt(id);
    let stored = spec.clone();
    save_settings(&app, |s| s.prompts.set(id, stored));
    spec
}

/// Trimmed; an empty key removes the stored one.
#[tauri::command]
pub fn set_api_key(app: AppHandle, key: String) {
    let key = key.trim().to_string();
    save_settings(&app, |s| s.api_key = key);
}

#[tauri::command]
pub fn set_english_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    if lang == Lang::En {
        return Err("English cannot be the target for English text".into());
    }
    save_settings(&app, |s| s.english_target = lang);
    Ok(())
}

/// Switches the translator hotkey. On a bad or taken shortcut the old one
/// stays registered and the error comes back.
#[tauri::command]
pub fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    let hotkey = hotkey.trim().to_string();
    Shortcut::from_str(&hotkey).map_err(|e| format!("Not a valid shortcut: {e}"))?;
    let old = app.state::<AppState>().settings().hotkey;
    if old == hotkey {
        return Ok(());
    }
    let shortcuts = app.global_shortcut();
    let _ = shortcuts.unregister(old.as_str());
    if let Err(e) = crate::register_hotkey(&app, &hotkey) {
        if let Err(back) = crate::register_hotkey(&app, &old) {
            log::error!("cannot restore hotkey {old}: {back}");
        }
        return Err(format!("Cannot use {hotkey}: {e}"));
    }
    save_settings(&app, |s| s.hotkey = hotkey);
    Ok(())
}
