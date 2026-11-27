//! Replacing the selection in the frontmost app: clipboard + ⌘V, then the
//! previous clipboard text comes back.

use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::clipboard_lock::ClipboardLock;
use crate::keys;

/// Time for the target app to read the clipboard before it is restored.
const RESTORE_AFTER: Duration = Duration::from_millis(300);

pub fn replace_selection(app: &AppHandle, text: &str) -> Result<(), String> {
    // Held until the previous clipboard is back, in the restore thread.
    let lock = ClipboardLock::acquire();
    let clipboard = app.clipboard();
    let saved = clipboard.read_text().ok();
    clipboard.write_text(text).map_err(|e| e.to_string())?;
    keys::command_v()?;
    let app = app.clone();
    thread::spawn(move || {
        let _lock = lock;
        thread::sleep(RESTORE_AFTER);
        let clipboard = app.clipboard();
        let restored = match saved {
            Some(text) => clipboard.write_text(text),
            None => clipboard.clear(),
        };
        if let Err(e) = restored {
            log::error!("cannot restore the clipboard: {e}");
        }
    });
    Ok(())
}
