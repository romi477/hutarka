//! The translator popup: a borderless, non-activating NSPanel near the mouse.
//! It becomes key (gets the keyboard) without activating Hutarka, so the app
//! the text came from stays frontmost. Panel setup follows Handy's
//! `overlay.rs` (MIT).

// The panel_event! macro requires an explicit `-> ()`.
#![allow(clippy::unused_unit)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    AppHandle, Emitter, LogicalSize, Manager, PhysicalPosition, Position, Size, WebviewUrl,
};
use tauri_nspanel::{
    CollectionBehavior, ManagerExt, PanelBuilder, PanelLevel, StyleMask, tauri_panel,
};

pub const LABEL: &str = "popup";
/// Window width when it opens: the 520 pt card plus room for its shadow
/// (2 × 20). The page asks for a wider window for long text.
const WIDTH: f64 = 560.0;
const MIN_WIDTH: f64 = 360.0;
const INITIAL_HEIGHT: f64 = 200.0;
const MIN_HEIGHT: f64 = 60.0;
/// Gap between the mouse and the popup's top-left corner, in points.
const OFFSET: f64 = 8.0;

/// Pinned: losing the keyboard (a click elsewhere, another Space) does not
/// close the popup, and it shows on every Space. Explicit closing (Esc,
/// Close, Copy, Replace) unpins it.
static PINNED: AtomicBool = AtomicBool::new(false);

fn behavior(pinned: bool) -> CollectionBehavior {
    let b = CollectionBehavior::new();
    // Unpinned it opens on the current Space and stays there; switching
    // Spaces makes it lose the keyboard, which closes it.
    let b = if pinned {
        b.can_join_all_spaces()
    } else {
        b.move_to_active_space()
    };
    b.full_screen_auxiliary()
}

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

/// Where the popup was opened (physical screen coordinates of the mouse), and
/// whether the user dragged it since.
#[derive(Default)]
pub struct Anchor(Mutex<Placed>);

#[derive(Default)]
pub struct Placed {
    mouse: Option<PhysicalPosition<f64>>,
    dragged: bool,
    side: Side,
}

/// A monitor's work area, physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

/// Which side of the mouse the popup keeps while it is open.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum Side {
    /// Just shown: pick below, or above when there is no room below.
    #[default]
    Fresh,
    /// Top edge just under the mouse; grows down, shifts up at the screen bottom.
    Below,
    /// Bottom edge just above the mouse; grows up.
    Above,
    /// Dragged by the user: top-left where it was dropped.
    Dropped(f64, f64),
}

/// Top-left corner for a window of `size`, and the side to keep from now on.
/// The side is chosen once, when the popup is shown, so later height changes
/// (streaming, the direction menu) never make it jump across the mouse.
/// Always inside the area as far as the size allows.
fn origin(
    mouse: (f64, f64),
    side: Side,
    size: (f64, f64),
    gap: f64,
    area: Area,
) -> ((f64, f64), Side) {
    let (w, h) = size;
    let below = mouse.1 + gap;
    let side = match side {
        Side::Fresh if below + h > area.bottom => Side::Above,
        Side::Fresh => Side::Below,
        kept => kept,
    };
    let (x, y) = match side {
        Side::Dropped(x, y) => (x, y.min(area.bottom - h)),
        Side::Above => (mouse.0 + gap, mouse.1 - gap - h),
        _ => (mouse.0 + gap, below.min(area.bottom - h)),
    };
    let x = x.min(area.right - w);
    ((x.max(area.left), y.max(area.top)), side)
}

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
        .collection_behavior(behavior(false))
        .build()?;
    let events = TranslatorPanelEvents::new();
    let handle = app.clone();
    // Click outside (another window becomes key): close, unless pinned.
    events.window_did_resign_key(move |_| {
        if !PINNED.load(Ordering::SeqCst) {
            hide(&handle);
        }
    });
    panel.set_event_handler(Some(events.as_ref()));
    panel.hide();
    Ok(())
}

/// Shows the popup at the mouse and gives it the keyboard. A pinned popup
/// stays where it is.
pub fn show_near_cursor(app: &AppHandle) {
    if PINNED.load(Ordering::SeqCst) {
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Ok(panel) = handle.get_webview_panel(LABEL) {
                panel.show_and_make_key();
            }
        });
        return;
    }
    if let Ok(cursor) = app.cursor_position()
        && let Ok(mut placed) = app.state::<Anchor>().0.lock()
    {
        *placed = Placed {
            mouse: Some(cursor),
            dragged: false,
            side: Side::Fresh,
        };
    }
    place(app, INITIAL_HEIGHT, WIDTH);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.show_and_make_key();
        }
    });
}

/// Pins or unpins the popup and tells the page.
pub fn set_pinned(app: &AppHandle, pinned: bool) {
    if PINNED.swap(pinned, Ordering::SeqCst) == pinned {
        return;
    }
    let _ = app.emit_to(LABEL, "popup:pinned", pinned);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL) {
            panel.set_collection_behavior(behavior(pinned).value());
        }
    });
}

pub fn is_pinned() -> bool {
    PINNED.load(Ordering::SeqCst)
}

/// Closes the popup and unpins it.
pub fn hide(app: &AppHandle) {
    set_pinned(app, false);
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(panel) = handle.get_webview_panel(LABEL)
            && panel.is_visible()
        {
            panel.hide();
        }
    });
}

/// The page saw a drag start on the header: from now on the popup stays
/// where the user puts it.
pub fn mark_dragged(app: &AppHandle) {
    if let Ok(mut placed) = app.state::<Anchor>().0.lock() {
        placed.dragged = true;
    }
}

/// New content size from the page: resize, keep the popup on screen.
pub fn resize(app: &AppHandle, height: f64, width: f64) {
    place(app, height.max(MIN_HEIGHT), width.max(MIN_WIDTH));
}

/// Sizes the window (capped by the monitor) and positions it with
/// [`origin`].
fn place(app: &AppHandle, height: f64, width: f64) {
    let Some(window) = app.get_webview_window(LABEL) else {
        return;
    };
    let anchor = app.state::<Anchor>();
    let Ok(mut placed) = anchor.0.lock() else {
        return;
    };
    let Some(mouse) = placed.mouse else {
        return;
    };
    let monitor = app
        .monitor_from_point(mouse.x, mouse.y)
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        return;
    };
    let scale = monitor.scale_factor();
    let work = monitor.work_area();
    let area = Area {
        left: work.position.x as f64,
        top: work.position.y as f64,
        right: work.position.x as f64 + work.size.width as f64,
        bottom: work.position.y as f64 + work.size.height as f64,
    };
    let height = height.min(work.size.height as f64 / scale);
    let width = width.min(work.size.width as f64 / scale);
    let current = window.outer_position().ok();
    let side = match (placed.dragged, current) {
        (true, Some(now)) => Side::Dropped(now.x as f64, now.y as f64),
        _ => placed.side,
    };
    let ((x, y), side) = origin(
        (mouse.x, mouse.y),
        side,
        (width * scale, height * scale),
        OFFSET * scale,
        area,
    );
    let pos = PhysicalPosition {
        x: x.round() as i32,
        y: y.round() as i32,
    };
    let _ = window.set_size(Size::Logical(LogicalSize { width, height }));
    let _ = window.set_position(Position::Physical(pos));
    placed.side = side;
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = Area {
        left: 0.0,
        top: 0.0,
        right: 1000.0,
        bottom: 800.0,
    };
    const SIZE: (f64, f64) = (400.0, 300.0);

    #[test]
    fn fresh_opens_below_right_of_mouse() {
        assert_eq!(
            origin((100.0, 100.0), Side::Fresh, SIZE, 8.0, AREA),
            ((108.0, 108.0), Side::Below)
        );
    }

    #[test]
    fn fresh_opens_above_when_no_room_below() {
        assert_eq!(
            origin((100.0, 700.0), Side::Fresh, SIZE, 8.0, AREA),
            ((108.0, 392.0), Side::Above)
        );
    }

    #[test]
    fn above_taller_than_room_sticks_to_top() {
        let (pos, _) = origin((100.0, 300.0), Side::Above, (400.0, 700.0), 8.0, AREA);
        assert_eq!(pos, (108.0, 0.0));
    }

    #[test]
    fn below_growing_past_bottom_shifts_up_instead_of_flipping() {
        // The direction menu adds height: the popup must not jump above the mouse.
        assert_eq!(
            origin((100.0, 500.0), Side::Below, (400.0, 400.0), 8.0, AREA),
            ((108.0, 400.0), Side::Below)
        );
    }

    #[test]
    fn above_keeps_its_bottom_next_to_the_mouse() {
        assert_eq!(
            origin((100.0, 700.0), Side::Above, (400.0, 200.0), 8.0, AREA),
            ((108.0, 492.0), Side::Above)
        );
    }

    #[test]
    fn dropped_stays_where_dropped() {
        let (pos, _) = origin((100.0, 100.0), Side::Dropped(500.0, 200.0), SIZE, 8.0, AREA);
        assert_eq!(pos, (500.0, 200.0));
    }

    #[test]
    fn dropped_grows_up_at_the_bottom() {
        let (pos, _) = origin((100.0, 100.0), Side::Dropped(500.0, 600.0), SIZE, 8.0, AREA);
        assert_eq!(pos, (500.0, 500.0));
    }
}
