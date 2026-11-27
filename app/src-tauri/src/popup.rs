//! The translator popup: a borderless, non-activating NSPanel near the mouse.
//! It becomes key (gets the keyboard) without activating Hutarka, so the app
//! the text came from stays frontmost. Panel setup follows Handy's
//! `overlay.rs` (MIT).

// The panel_event! macro requires an explicit `-> ()`.
#![allow(clippy::unused_unit)]

use std::sync::Mutex;

use tauri::{AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask, tauri_panel,
};

pub const LABEL: &str = "popup";
/// Window width: the 400 pt card plus room for its shadow.
const WIDTH: f64 = 440.0;
const INITIAL_HEIGHT: f64 = 200.0;
const MAX_HEIGHT: f64 = 600.0;
/// Gap between the mouse and the popup's top-left corner, in points.
const OFFSET: f64 = 8.0;

tauri_panel! {
    panel!(TranslatorPanel {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })

    panel_event!(TranslatorPanelEvents {
        window_did_resign_key(notification: &NSNotification) -> ()
    })
}

/// Where the popup was opened (physical screen coordinates of the mouse).
#[derive(Default)]
pub struct Anchor(Mutex<Option<PhysicalPosition<f64>>>);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let panel = PanelBuilder::<_, TranslatorPanel>::new(app, LABEL)
        .url(WebviewUrl::App("src/popup/index.html".into()))
        .title("Hutarka")
        .level(PanelLevel::Floating)
        .size(Size::Logical(LogicalSize {
            width: WIDTH,
            height: INITIAL_HEIGHT,
        }))
        .has_shadow(false)
        .transparent(true)
        .corner_radius(0.0)
        .style_mask(StyleMask::empty().borderless().nonactivating_panel())
        .with_window(|w| w.decorations(false).transparent(true))
        .collection_behavior(
            CollectionBehavior::new()
                .can_join_all_spaces()
                .full_screen_auxiliary(),
        )
        .build()?;
    let events = TranslatorPanelEvents::new();
    let handle = app.clone();
    // Click outside (another window becomes key): close.
    events.window_did_resign_key(move |_| hide(&handle));
    panel.set_event_handler(Some(events.as_ref()));
    panel.hide();
    Ok(())
}

/// Shows the popup at the mouse and gives it the keyboard.
pub fn show_near_cursor(app: &AppHandle) {
    if let Ok(cursor) = app.cursor_position()
        && let Ok(mut anchor) = app.state::<Anchor>().0.lock()
    {
        *anchor = Some(cursor);
    }
    place(app, INITIAL_HEIGHT);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.show_and_make_key();
        }
    });
}

pub fn hide(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL)
            && panel.is_visible()
        {
            panel.hide();
        }
    });
}

/// New content height from the page: resize, keep the popup on screen.
pub fn resize(app: &AppHandle, height: f64) {
    place(app, height.clamp(60.0, MAX_HEIGHT));
}

/// Sizes the window and puts it below-right of the anchor, moved left or up
/// as needed to stay inside the anchor's monitor.
fn place(app: &AppHandle, height: f64) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let anchor = app.state::<Anchor>().0.lock().ok().and_then(|a| *a);
    let Some(anchor) = anchor else {
        return;
    };
    let monitor = app
        .monitor_from_point(anchor.x, anchor.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let scale = monitor.scale_factor();
    let (w, h, gap) = (WIDTH * scale, height * scale, OFFSET * scale);
    let area = monitor.work_area();
    let (left, top) = (area.position.x as f64, area.position.y as f64);
    let (right, bottom) = (left + area.size.width as f64, top + area.size.height as f64);
    let mut x = anchor.x + gap;
    let mut y = anchor.y + gap;
    if x + w > right {
        x = (right - w).max(left);
    }
    if y + h > bottom {
        // Not enough room below the mouse: open above it.
        y = (anchor.y - gap - h).max(top);
    }
    let _ = window.set_size(Size::Logical(LogicalSize {
        width: WIDTH,
        height,
    }));
    let _ = window.set_position(Position::Physical(PhysicalPosition {
        x: x.round() as i32,
        y: y.round() as i32,
    }));
}
