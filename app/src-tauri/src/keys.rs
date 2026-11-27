//! Synthetic ⌘C / ⌘V. Uses physical key codes so it works with any keyboard
//! layout (macOS maps ⌘ shortcuts by key position on non-Latin layouts).

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// ANSI key codes (Carbon `kVK_ANSI_C`, `kVK_ANSI_V`).
const KEY_C: u32 = 8;
const KEY_V: u32 = 9;

fn command(code: u32) -> Result<(), String> {
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| e.to_string())?;
    let typed = enigo.raw(code as u16, Direction::Click);
    // Always release ⌘, even if the key press failed.
    let released = enigo.key(Key::Meta, Direction::Release);
    typed.map_err(|e| e.to_string())?;
    released.map_err(|e| e.to_string())
}

pub fn command_c() -> Result<(), String> {
    command(KEY_C)
}

pub fn command_v() -> Result<(), String> {
    command(KEY_V)
}
