//! What to send for a selection: target language, kind of answer, prompt text.
//! Pure logic, no macOS calls.

mod lang;
mod prompt;

pub use lang::Lang;
pub use prompt::{Kind, TextError, kind_for, prepare_text, render};
