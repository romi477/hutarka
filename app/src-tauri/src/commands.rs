//! Commands of the Settings window.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::lookup::{AppState, save_settings};
use crate::settings::{PromptId, PromptSpec, Settings, SettingsView, default_prompt};
use crate::translator::Lang;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    SettingsView::from(&app.state::<AppState>().settings())
}

#[tauri::command]
pub fn set_prompt(app: AppHandle, id: PromptId, spec: PromptSpec) {
    save_settings(&app, |s| s.set_prompt(id, spec));
}

#[tauri::command]
pub fn reset_prompt(app: AppHandle, id: PromptId) -> PromptSpec {
    let spec = default_prompt(id);
    let stored = spec.clone();
    save_settings(&app, |s| s.set_prompt(id, stored));
    spec
}

/// Trimmed; an empty key removes the stored one.
#[tauri::command]
pub fn set_api_key(app: AppHandle, key: String) {
    let key = key.trim().to_string();
    save_settings(&app, |s| s.api_key = key);
}

/// Adds a target-language card at the end.
#[tauri::command]
pub fn add_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    change_settings(&app, |s| s.add_target(lang))
}

/// Removes a card; the default one is refused.
#[tauri::command]
pub fn remove_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    change_settings(&app, |s| s.remove_target(lang))
}

/// Makes `lang` the language every lookup starts with.
#[tauri::command]
pub fn set_default_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    change_settings(&app, |s| s.set_default_target(lang))
}

/// Applies a change that may be refused; saves only when it is not.
fn change_settings(
    app: &AppHandle,
    change: impl FnOnce(&mut Settings) -> Result<(), String>,
) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned".to_string())?;
    let mut next = settings.clone();
    change(&mut next)?;
    *settings = next;
    crate::settings::save(app, &settings)
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
