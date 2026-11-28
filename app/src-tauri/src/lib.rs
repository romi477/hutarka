//! Hutarka: menu bar app with the selection translator.

mod clipboard_lock;
mod commands;
mod journal;
mod keys;
mod lookup;
mod paste;
mod pasteboard;
mod popup;
mod selection;
mod session;
mod settings;
mod translator;
mod tray;
mod windows;

use tauri::menu::{Menu, PredefinedMenuItem, Submenu};
use tauri::{ActivationPolicy, AppHandle, Manager, Wry};
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
            lookup::copy_text,
            lookup::clipboard_text,
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
            commands::add_target,
            commands::remove_target,
            commands::set_default_target,
            commands::set_hotkey,
        ])
        .setup(|app| {
            app.set_activation_policy(ActivationPolicy::Accessory);
            let handle = app.handle();
            handle.set_menu(edit_menu(handle)?)?;
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

/// A menu bar app shows no menus, but without an Edit menu macOS gives the
/// windows no ⌘C, ⌘V, ⌘X, ⌘A, ⌘Z. This hidden one makes them work in
/// Settings (the popup does them itself: it never makes the app active).
fn edit_menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let edit = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;
    let app_menu = Submenu::with_items(
        app,
        "Hutarka",
        true,
        &[&PredefinedMenuItem::quit(app, None)?],
    )?;
    Menu::with_items(app, &[&app_menu, &edit])
}
