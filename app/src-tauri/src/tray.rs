//! Menu bar icon and its menu. Pattern from Handy's `tray.rs` (MIT).

use tauri::AppHandle;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;

use crate::windows;

const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");

pub fn build(app: &AppHandle) -> tauri::Result<()> {
    // The selection if there is one, otherwise an empty field to type into.
    let translate = MenuItem::with_id(app, "translate", "Translate", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let quit = MenuItem::with_id(app, "quit", "Quit Hutarka", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(
        app,
        &[
            &translate,
            &PredefinedMenuItem::separator(app)?,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;
    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(TRAY_ICON)?)
        .icon_as_template(true)
        .tooltip("Hutarka")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "translate" => crate::lookup::trigger(app),
            "settings" => windows::open_settings(app, None),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
