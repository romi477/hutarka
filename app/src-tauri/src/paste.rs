//! Replacing the selection in the frontmost app: clipboard + ⌘V, then the
//! previous clipboard (all of it) comes back, unless something new was
//! copied in the meantime.

use std::thread;
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::clipboard_lock::ClipboardLock;
use crate::{keys, pasteboard};

/// Time for the target app to read the clipboard before it is restored.
const RESTORE_AFTER: Duration = Duration::from_millis(300);

pub fn replace_selection(app: &AppHandle, text: &str) -> Result<(), String> {
    // Held until the previous clipboard is back, in the restore thread.
    let lock = ClipboardLock::acquire();
    // Everything on the clipboard (images, files, rich text), not just text.
    let saved = pasteboard::save();
    if let Err(e) = app.clipboard().write_text(text) {
        pasteboard::restore(&saved);
        return Err(e.to_string());
    }
    // Pasting does not change the count; a copy by the user meanwhile does.
    let ours = pasteboard::change_count();
    if let Err(e) = keys::command_v() {
        pasteboard::restore(&saved);
        return Err(e);
    }
    thread::spawn(move || {
        let _lock = lock;
        thread::sleep(RESTORE_AFTER);
        if !pasteboard::restore_if_unchanged(&saved, ours) {
            log::info!("clipboard changed after Replace: the new content is kept");
        }
    });
    Ok(())
}
