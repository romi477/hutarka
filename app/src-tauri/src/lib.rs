//! Hutarka: menu bar app with the selection translator.

mod clipboard_lock;
mod commands;
mod journal;
mod keys;
mod lookup;
mod paste;
mod popup;
mod selection;
mod session;
mod settings;
mod translator;
mod tray;
mod windows;

use tauri::{ActivationPolicy, AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::lookup::AppState;

/// Makes `hotkey` start a lookup. Fires on release, so the hotkey's own keys
/// are up before Hutarka sends ⌘C to the source app.
pub fn register_hotkey(app: &AppHandle, hotkey: &str) -> Result<(), String> {
    app.global_shortcut()
        .on_shortcut(hotkey, |app, _shortcut, event| {
            if event.state == ShortcutState::Released {
                lookup::trigger(app);
            }
        })
        .map_err(|e| e.to_string())
}

pub fn run() {
    let result = tauri::Builder::default()
        // ~/Library/Logs/dev.hutarka.app/Hutarka.log, plus stdout.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("Hutarka".into()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                ])
                .build(),
        )
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_nspanel::init())
        .manage(popup::Anchor::default())
        .invoke_handler(tauri::generate_handler![
            lookup::popup_snapshot,
            lookup::rerun,
            lookup::translate_text,
            lookup::copy_result,
            lookup::replace_result,
            lookup::close_popup,
            lookup::resize_popup,
            lookup::popup_dragged,
            lookup::open_settings,
            lookup::open_accessibility,
            commands::get_settings,
            commands::set_prompt,
            commands::reset_prompt,
            commands::set_api_key,
            commands::set_english_target,
            commands::set_hotkey,
        ])
        .setup(|app| {
            app.set_activation_policy(ActivationPolicy::Accessory);
            let handle = app.handle();
            let settings = settings::load(handle);
            let hotkey = settings.hotkey.clone();
            app.manage(AppState::new(settings));
            popup::create(handle)?;
            tray::build(handle)?;
            if let Err(e) = register_hotkey(handle, &hotkey) {
                log::error!("cannot register hotkey {hotkey}: {e}");
            }
            Ok(())
        })
        .build(tauri::generate_context!());
    match result {
        // Closing the last window must not quit a menu bar app.
        Ok(app) => app.run(|_, event| {
            if let tauri::RunEvent::ExitRequested {
                api, code: None, ..
            } = event
            {
                api.prevent_exit();
            }
        }),
        Err(e) => eprintln!("hutarka: cannot start: {e}"),
    }
}
