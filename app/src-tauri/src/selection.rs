//! Reading the selected text of the frontmost app: Accessibility API first,
//! then a ⌘C round trip through the clipboard (all of it restored afterwards).

use std::ptr::NonNull;
use std::thread;
use std::time::{Duration, Instant};

use objc2_app_kit::NSWorkspace;
use objc2_application_services::{AXError, AXIsProcessTrusted, AXUIElement};
use objc2_core_foundation::{CFRetained, CFString, CFType};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::clipboard_lock::ClipboardLock;
use crate::{keys, pasteboard};

/// Written to the clipboard before ⌘C: still there afterwards means the app
/// copied nothing.
const SENTINEL: &str = "\u{2063}hutarka-selection-probe\u{2063}";
const COPY_WAIT: Duration = Duration::from_millis(300);
const POLL: Duration = Duration::from_millis(20);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionError {
    /// Hutarka has no Accessibility permission.
    NotTrusted,
    /// Nothing is selected (or the app exposes no selection).
    Nothing,
}

pub fn accessibility_trusted() -> bool {
    // SAFETY: plain query, no arguments.
    unsafe { AXIsProcessTrusted() }
}

/// System Settings › Privacy & Security › Accessibility.
pub fn open_accessibility_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .status();
}

/// Where a selection was read: the frontmost app and its focused window.
/// Replace pastes only while both are still in front.
pub struct Origin {
    pid: i32,
    window: Option<CFRetained<AXUIElement>>,
}

// SAFETY: an AXUIElement is an immutable reference to a UI element, usable
// from any thread; Origin only compares it.
unsafe impl Send for Origin {}

impl Origin {
    /// Same app, and the same window when both windows are known.
    pub fn same_place(&self, now: &Origin) -> bool {
        self.pid == now.pid
            && match (&self.window, &now.window) {
                (Some(then), Some(now)) => **then == **now,
                _ => true,
            }
    }
}

/// The frontmost (active) app and its focused window. The popup is a
/// non-activating panel, so it never counts as frontmost; None when Hutarka
/// itself is in front (its own window cannot be pasted into).
pub fn current_origin() -> Option<Origin> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let pid = app.processIdentifier();
    if pid == std::process::id() as i32 {
        return None;
    }
    // SAFETY: returns a new reference to the app's element.
    let element = unsafe { AXUIElement::new_application(pid) };
    let window =
        attribute(&element, "AXFocusedWindow").and_then(|w| w.downcast::<AXUIElement>().ok());
    Some(Origin { pid, window })
}

fn clipboard_changed(sentinel: &str, now: Option<&str>) -> bool {
    matches!(now, Some(text) if text != sentinel)
}

pub fn read_selection(app: &AppHandle) -> Result<String, SelectionError> {
    if !accessibility_trusted() {
        return Err(SelectionError::NotTrusted);
    }
    if let Some(text) = ax_selected_text().filter(|t| !t.trim().is_empty()) {
        return Ok(text);
    }
    copy_selection(app).ok_or(SelectionError::Nothing)
}

/// One attribute of an element, retained.
fn attribute(element: &AXUIElement, name: &str) -> Option<CFRetained<CFType>> {
    let name = CFString::from_str(name);
    let mut value: *const CFType = std::ptr::null();
    // SAFETY: `value` is a valid out pointer; on success it holds a +1
    // reference that CFRetained takes over.
    let err = unsafe { element.copy_attribute_value(&name, NonNull::from(&mut value)) };
    if err != AXError::Success {
        return None;
    }
    NonNull::new(value.cast_mut()).map(|ptr| unsafe { CFRetained::from_raw(ptr) })
}

fn ax_selected_text() -> Option<String> {
    // SAFETY: returns a new reference to the system-wide element.
    let system = unsafe { AXUIElement::new_system_wide() };
    let focused = attribute(&system, "AXFocusedUIElement")?
        .downcast::<AXUIElement>()
        .ok()?;
    let text = attribute(&focused, "AXSelectedText")?
        .downcast::<CFString>()
        .ok()?;
    Some(text.to_string())
}

/// ⌘C fallback for apps without AX text (browsers, Electron, terminals).
fn copy_selection(app: &AppHandle) -> Option<String> {
    let _lock = ClipboardLock::acquire();
    let clipboard = app.clipboard();
    // Everything on the clipboard (images, files, rich text), not just text.
    let saved = pasteboard::save();
    if clipboard.write_text(SENTINEL).is_err() {
        pasteboard::restore(&saved);
        return None;
    }
    let copied = keys::command_c().ok().and_then(|_| {
        let start = Instant::now();
        loop {
            let now = clipboard.read_text().ok();
            if clipboard_changed(SENTINEL, now.as_deref()) {
                break now;
            }
            if start.elapsed() > COPY_WAIT {
                break None;
            }
            thread::sleep(POLL);
        }
    });
    pasteboard::restore(&saved);
    copied.filter(|t| !t.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn origin(pid: i32, window: Option<i32>) -> Origin {
        // Application elements of made-up pids stand in for windows: equal
        // exactly when built from the same pid.
        Origin {
            pid,
            window: window.map(|w| unsafe { AXUIElement::new_application(w) }),
        }
    }

    #[test]
    fn replace_needs_the_same_app_and_window() {
        assert!(origin(10, Some(1)).same_place(&origin(10, Some(1))));
        assert!(!origin(10, Some(1)).same_place(&origin(11, Some(1))));
        assert!(!origin(10, Some(1)).same_place(&origin(10, Some(2))));
        // A window AX does not expose: the app decides.
        assert!(origin(10, None).same_place(&origin(10, Some(2))));
    }

    #[test]
    fn sentinel_unchanged_means_nothing() {
        assert!(!clipboard_changed(SENTINEL, Some(SENTINEL)));
        assert!(!clipboard_changed(SENTINEL, None));
        assert!(clipboard_changed(SENTINEL, Some("leverage")));
    }
}
