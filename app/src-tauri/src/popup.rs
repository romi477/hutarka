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
/// Window width: the 520 pt card plus room for its shadow (2 × 20).
const WIDTH: f64 = 560.0;
const INITIAL_HEIGHT: f64 = 200.0;
const MIN_HEIGHT: f64 = 60.0;
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

/// Where the popup was opened (physical screen coordinates of the mouse), and
/// where Hutarka last put it: a different position means the user dragged it.
#[derive(Default)]
pub struct Anchor(Mutex<Placed>);

#[derive(Default)]
pub struct Placed {
    mouse: Option<PhysicalPosition<f64>>,
    last: Option<PhysicalPosition<i32>>,
}

/// A monitor's work area, physical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Area {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

/// Top-left corner for a window of `size`. Not moved by the user: below-right
/// of the mouse, flipped above it when there is no room below. Moved: where
/// the user dropped it, shifted up if it grew past the bottom. Always inside
/// the area as far as the size allows.
fn origin(
    mouse: (f64, f64),
    dropped: Option<(f64, f64)>,
    size: (f64, f64),
    gap: f64,
    area: Area,
) -> (f64, f64) {
    let (w, h) = size;
    let (mut x, mut y) = match dropped {
        Some(pos) => pos,
        None => (mouse.0 + gap, mouse.1 + gap),
    };
    if x + w > area.right {
        x = area.right - w;
    }
    if y + h > area.bottom {
        y = match dropped {
            Some(_) => area.bottom - h,
            None => mouse.1 - gap - h,
        };
    }
    (x.max(area.left), y.max(area.top))
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
        && let Ok(mut placed) = app.state::<Anchor>().0.lock()
    {
        *placed = Placed {
            mouse: Some(cursor),
            last: None,
        };
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
    place(app, height.max(MIN_HEIGHT));
}

/// Sizes the window (height capped by the monitor) and positions it with
/// [`origin`].
fn place(app: &AppHandle, height: f64) {
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
    let current = window.outer_position().ok();
    let dropped = match (placed.last, current) {
        (Some(last), Some(now)) if last != now => Some((now.x as f64, now.y as f64)),
        _ => None,
    };
    let (x, y) = origin(
        (mouse.x, mouse.y),
        dropped,
        (WIDTH * scale, height * scale),
        OFFSET * scale,
        area,
    );
    let pos = PhysicalPosition {
        x: x.round() as i32,
        y: y.round() as i32,
    };
    let _ = window.set_size(Size::Logical(LogicalSize {
        width: WIDTH,
        height,
    }));
    let _ = window.set_position(Position::Physical(pos));
    placed.last = Some(pos);
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = Area { left: 0.0, top: 0.0, right: 1000.0, bottom: 800.0 };

    #[test]
    fn opens_below_right_of_mouse() {
        assert_eq!(origin((100.0, 100.0), None, (400.0, 300.0), 8.0, AREA), (108.0, 108.0));
    }

    #[test]
    fn opens_above_when_no_room_below() {
        assert_eq!(origin((100.0, 700.0), None, (400.0, 300.0), 8.0, AREA), (108.0, 392.0));
    }

    #[test]
    fn taller_than_room_above_sticks_to_top() {
        assert_eq!(origin((100.0, 300.0), None, (400.0, 700.0), 8.0, AREA), (108.0, 0.0));
    }

    #[test]
    fn moved_by_user_stays_where_dropped() {
        let pos = origin((100.0, 100.0), Some((500.0, 200.0)), (400.0, 300.0), 8.0, AREA);
        assert_eq!(pos, (500.0, 200.0));
    }

    #[test]
    fn moved_by_user_grows_up_at_the_bottom() {
        let pos = origin((100.0, 100.0), Some((500.0, 600.0)), (400.0, 300.0), 8.0, AREA);
        assert_eq!(pos, (500.0, 500.0));
    }
}
