use serde::{Deserialize, Serialize};

use super::Direction;

/// What the answer should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// Plain translation.
    Text,
    /// Dictionary entry for a word or short phrase.
    Word,
    /// Deep explanation with example sentences.
    Explain,
}

/// Longest selection sent to the model.
pub const MAX_CHARS: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextError {
    Blank,
    TooLong(usize),
}

impl std::fmt::Display for TextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextError::Blank => write!(f, "Nothing selected"),
            TextError::TooLong(n) => {
                write!(f, "Selection too long ({n} characters, limit {MAX_CHARS})")
            }
        }
    }
}

/// Checks a selection; returns it unchanged when it can be sent.
pub fn prepare_text(raw: &str) -> Result<String, TextError> {
    if raw.trim().is_empty() {
        return Err(TextError::Blank);
    }
    let count = raw.chars().count();
    if count > MAX_CHARS {
        return Err(TextError::TooLong(count));
    }
    Ok(raw.to_string())
}

/// Explain when asked; a word or short phrase (up to three words, 40
/// characters, no sentence punctuation) gets a dictionary entry.
pub fn kind_for(text: &str, explain: bool) -> Kind {
    if explain {
        return Kind::Explain;
    }
    let trimmed = text.trim();
    let short = trimmed.split_whitespace().count() <= 3 && trimmed.chars().count() <= 40;
    if short && !trimmed.contains(['.', '!', '?']) {
        Kind::Word
    } else {
        Kind::Text
    }
}

/// Fills `{from}` and `{to}` with the direction's language names.
pub fn render(template: &str, dir: Direction) -> String {
    template
        .replace("{from}", dir.from().name())
        .replace("{to}", dir.to().name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translator::Direction;

    #[test]
    fn explain_wins() {
        assert_eq!(kind_for("leverage", true), Kind::Explain);
        assert_eq!(kind_for("We can leverage it.", true), Kind::Explain);
    }

    #[test]
    fn single_word_is_word() {
        assert_eq!(kind_for("leverage", false), Kind::Word);
    }

    #[test]
    fn three_words_is_word() {
        assert_eq!(kind_for("gain the upper", false), Kind::Word);
    }

    #[test]
    fn four_words_is_text() {
        assert_eq!(kind_for("gain the upper hand", false), Kind::Text);
    }

    #[test]
    fn sentence_is_text() {
        assert_eq!(kind_for("We can leverage it.", false), Kind::Text);
        assert_eq!(kind_for("Really?", false), Kind::Text);
    }

    #[test]
    fn prepare_text_rejects_blank() {
        assert_eq!(prepare_text("  \n"), Err(TextError::Blank));
        assert_eq!(TextError::Blank.to_string(), "Nothing selected");
    }

    #[test]
    fn prepare_text_rejects_over_limit() {
        let long = "a".repeat(MAX_CHARS + 1);
        assert_eq!(prepare_text(&long), Err(TextError::TooLong(20_001)));
        assert_eq!(
            TextError::TooLong(20_001).to_string(),
            "Selection too long (20001 characters, limit 20000)"
        );
    }

    #[test]
    fn prepare_text_keeps_text_as_is() {
        assert_eq!(prepare_text(" Привет\n"), Ok(" Привет\n".to_string()));
        assert!(prepare_text(&"я".repeat(MAX_CHARS)).is_ok());
    }

    #[test]
    fn render_fills_placeholders() {
        assert_eq!(
            render("{from} to {to}, {to}", Direction::EnPl),
            "English to Polish, Polish"
        );
    }

    #[test]
    fn kind_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&Kind::Explain).unwrap(),
            "\"explain\""
        );
    }
}
