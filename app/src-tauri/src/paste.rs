//! Replacing the selection in the frontmost app: clipboard + ⌘V, then the
//! previous clipboard (all of it) comes back.

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
    let pasted = app
        .clipboard()
        .write_text(text)
        .map_err(|e| e.to_string())
        .and_then(|_| keys::command_v());
    if let Err(e) = pasted {
        pasteboard::restore(&saved);
        return Err(e);
    }
    thread::spawn(move || {
        let _lock = lock;
        thread::sleep(RESTORE_AFTER);
        pasteboard::restore(&saved);
    });
    Ok(())
}
