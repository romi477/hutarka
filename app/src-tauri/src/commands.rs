//! Commands of the Settings window.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

use crate::lookup::{AppState, save_settings};
use crate::settings::{PromptId, PromptSpec, SettingsView, default_prompt};
use crate::translator::Lang;

#[tauri::command]
pub fn get_settings(app: AppHandle) -> SettingsView {
    let mut view = SettingsView::from(&app.state::<AppState>().settings());
    view.hotkey_active = app.global_shortcut().is_registered(view.hotkey.as_str());
    view
}

#[tauri::command]
pub fn set_prompt(app: AppHandle, id: PromptId, spec: PromptSpec) -> Result<(), String> {
    save_settings(&app, |s| {
        s.set_prompt(id, spec);
        Ok(())
    })
}

#[tauri::command]
pub fn reset_prompt(app: AppHandle, id: PromptId) -> Result<PromptSpec, String> {
    let spec = default_prompt(id);
    let stored = spec.clone();
    save_settings(&app, |s| {
        s.set_prompt(id, stored);
        Ok(())
    })?;
    Ok(spec)
}

/// Trimmed; an empty key removes the stored one.
#[tauri::command]
pub fn set_api_key(app: AppHandle, key: String) -> Result<(), String> {
    let key = key.trim().to_string();
    save_settings(&app, |s| {
        s.api_key = key;
        Ok(())
    })
}

/// Adds a target-language card at the end.
#[tauri::command]
pub fn add_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    save_settings(&app, |s| s.add_target(lang))
}

/// Removes a card; the default one is refused.
#[tauri::command]
pub fn remove_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    save_settings(&app, |s| s.remove_target(lang))
}

/// Makes `lang` the language every lookup starts with.
#[tauri::command]
pub fn set_default_target(app: AppHandle, lang: Lang) -> Result<(), String> {
    save_settings(&app, |s| s.set_default_target(lang))
}

/// Switches the translator hotkey. On a bad or taken shortcut, or a failed
/// save, the old one stays registered and the error comes back. Saving the
/// same hotkey registers it again when it is not registered (it was taken
/// when Hutarka started).
#[tauri::command]
pub fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    let hotkey = hotkey.trim().to_string();
    Shortcut::from_str(&hotkey).map_err(|e| format!("Not a valid shortcut: {e}"))?;
    let old = app.state::<AppState>().settings().hotkey;
    let shortcuts = app.global_shortcut();
    if old == hotkey {
        if shortcuts.is_registered(hotkey.as_str()) {
            return Ok(());
        }
        return crate::register_hotkey(&app, &hotkey)
            .map_err(|e| format!("Cannot use {hotkey}: {e}"));
    }
    let restore_old = || {
        if let Err(back) = crate::register_hotkey(&app, &old) {
            log::error!("cannot restore hotkey {old}: {back}");
        }
    };
    let _ = shortcuts.unregister(old.as_str());
    if let Err(e) = crate::register_hotkey(&app, &hotkey) {
        restore_old();
        return Err(format!("Cannot use {hotkey}: {e}"));
    }
    let new = hotkey.clone();
    if let Err(e) = save_settings(&app, |s| {
        s.hotkey = new;
        Ok(())
    }) {
        let _ = shortcuts.unregister(hotkey.as_str());
        restore_old();
        return Err(e);
    }
    Ok(())
}
