//! What to send for a selection: direction, kind of answer, prompt text.
//! Pure logic, no macOS calls.

mod lang;
mod prompt;

pub use lang::{Direction, Lang, auto_direction, detect_lang};
pub use prompt::{Kind, MAX_CHARS, TextError, kind_for, prepare_text, render};
