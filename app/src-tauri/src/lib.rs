//! Hutarka: menu bar app with the selection translator.

#[allow(dead_code, unused_imports)] // wired up in the popup flow (step 02, task 7)
mod translator;
mod tray;
mod windows;

use tauri::ActivationPolicy;

pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_nspanel::init())
        .setup(|app| {
            app.set_activation_policy(ActivationPolicy::Accessory);
            tray::build(app.handle())?;
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
