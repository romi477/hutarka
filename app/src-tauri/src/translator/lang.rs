use serde::{Deserialize, Serialize};

/// Languages of the selection translator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Pl,
    Ru,
}

impl Lang {
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Pl => "Polish",
            Lang::Ru => "Russian",
        }
    }
}

/// A translation direction; only these four exist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Direction {
    #[serde(rename = "en-ru")]
    EnRu,
    #[serde(rename = "en-pl")]
    EnPl,
    #[serde(rename = "pl-en")]
    PlEn,
    #[serde(rename = "ru-en")]
    RuEn,
}

impl Direction {
    pub fn from(self) -> Lang {
        match self {
            Direction::EnRu | Direction::EnPl => Lang::En,
            Direction::PlEn => Lang::Pl,
            Direction::RuEn => Lang::Ru,
        }
    }

    pub fn to(self) -> Lang {
        match self {
            Direction::EnRu => Lang::Ru,
            Direction::EnPl => Lang::Pl,
            Direction::PlEn | Direction::RuEn => Lang::En,
        }
    }
}

const POLISH_LETTERS: &str = "ąćęłńóśźżĄĆĘŁŃÓŚŹŻ";

/// Common Polish words as people type them without diacritics.
const POLISH_WORDS: &[&str] = &[
    "albo", "bardzo", "bez", "byl", "bylo", "chce", "czesc", "czlowiek", "dla", "dlaczego",
    "dobry", "dobrze", "dom", "dzien", "dzieki", "dziekuje", "dzis", "gdzie", "jak", "jest",
    "jestem", "jestes", "jestesmy", "jutro", "juz", "kiedy", "kot", "ktora", "ktore", "ktory",
    "mam", "masz", "miasto", "moze", "nie", "oraz", "pies", "praca", "prosze", "przez", "przy",
    "rok", "samochod", "sie", "tak", "teraz", "tez", "tylko", "wczoraj", "wiem", "woda",
    "wszystko",
];

/// Letter pairs frequent in Polish and rare in English.
const POLISH_DIGRAPHS: &[&str] = &["rz", "sz", "cz", "dz"];

/// Polish typed without diacritics: at least half of the words are common
/// Polish words or carry a Polish digraph. Short text is where `whatlang`
/// guesses badly.
fn looks_polish(text: &str) -> bool {
    let words: Vec<String> = text
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    if words.is_empty() {
        return false;
    }
    let polish = words
        .iter()
        .filter(|w| {
            POLISH_WORDS.contains(&w.as_str()) || POLISH_DIGRAPHS.iter().any(|d| w.contains(d))
        })
        .count();
    polish * 2 >= words.len()
}

/// Any Cyrillic letter means Russian; Polish letters, common Polish words or
/// `whatlang` mean Polish; anything else is English.
pub fn detect_lang(text: &str) -> Lang {
    if text.chars().any(|c| matches!(c, '\u{0400}'..='\u{04FF}')) {
        return Lang::Ru;
    }
    if text.chars().any(|c| POLISH_LETTERS.contains(c))
        || looks_polish(text)
        || whatlang::detect_lang(text) == Some(whatlang::Lang::Pol)
    {
        return Lang::Pl;
    }
    Lang::En
}

/// English goes to the remembered English target; the rest to English.
pub fn auto_direction(text: &str, english_target: Lang) -> Direction {
    match detect_lang(text) {
        Lang::En if english_target == Lang::Pl => Direction::EnPl,
        Lang::En => Direction::EnRu,
        Lang::Pl => Direction::PlEn,
        Lang::Ru => Direction::RuEn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polish_without_diacritics_is_polish() {
        for text in [
            "dziekuje",
            "bardzo dobrze",
            "samochod",
            "miasto",
            "kot",
            "dom",
            "czesc, jak sie masz",
        ] {
            assert_eq!(detect_lang(text), Lang::Pl, "{text}");
        }
    }

    #[test]
    fn english_with_no_polish_words_stays_english() {
        for text in [
            "hello world",
            "the cat is at home",
            "leverage",
            "domain",
            "cooperation",
            "pizza",
        ] {
            assert_eq!(detect_lang(text), Lang::En, "{text}");
        }
    }

    #[test]
    fn cyrillic_is_russian() {
        assert_eq!(detect_lang("Привет, как дела"), Lang::Ru);
    }

    #[test]
    fn polish_is_detected() {
        assert_eq!(detect_lang("Dzień dobry, jak się masz?"), Lang::Pl);
    }

    #[test]
    fn latin_defaults_to_english() {
        assert_eq!(detect_lang("The new release reduces latency"), Lang::En);
    }

    #[test]
    fn english_follows_target() {
        assert_eq!(auto_direction("leverage", Lang::Pl), Direction::EnPl);
        assert_eq!(auto_direction("leverage", Lang::Ru), Direction::EnRu);
    }

    #[test]
    fn russian_goes_to_english() {
        assert_eq!(
            auto_direction("Нужно проверить задержку", Lang::Pl),
            Direction::RuEn
        );
    }

    #[test]
    fn polish_goes_to_english() {
        assert_eq!(
            auto_direction("Dzień dobry, jak się masz?", Lang::Ru),
            Direction::PlEn
        );
    }

    #[test]
    fn direction_serde_roundtrip() {
        assert_eq!(
            serde_json::to_string(&Direction::EnPl).unwrap(),
            "\"en-pl\""
        );
        assert_eq!(
            serde_json::from_str::<Direction>("\"ru-en\"").unwrap(),
            Direction::RuEn
        );
        assert_eq!(serde_json::to_string(&Lang::Ru).unwrap(), "\"ru\"");
    }

    #[test]
    fn direction_ends() {
        assert_eq!(Direction::PlEn.from(), Lang::Pl);
        assert_eq!(Direction::PlEn.to(), Lang::En);
        assert_eq!(Lang::Pl.name(), "Polish");
    }
}
