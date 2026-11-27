//! What to send for a selection: direction, kind of answer, prompt text.
//! Pure logic, no macOS calls.

mod lang;
mod prompt;

pub use lang::{Direction, Lang, auto_direction};
pub use prompt::{Kind, TextError, kind_for, prepare_text, render};
