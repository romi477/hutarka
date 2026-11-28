use serde::{Deserialize, Serialize};

/// Languages the translator can translate into. The source language is
/// never detected here: the model works it out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    En,
    Ru,
    Be,
    Pl,
    De,
    Es,
    El,
    It,
    Fr,
}

impl Lang {
    /// Every language, in the order the Settings list offers them.
    pub const ALL: [Lang; 9] = [
        Lang::En,
        Lang::Ru,
        Lang::Be,
        Lang::Pl,
        Lang::De,
        Lang::Es,
        Lang::El,
        Lang::It,
        Lang::Fr,
    ];

    /// English name, used in prompts.
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Ru => "Russian",
            Lang::Be => "Belarusian",
            Lang::Pl => "Polish",
            Lang::De => "German",
            Lang::Es => "Spanish",
            Lang::El => "Greek",
            Lang::It => "Italian",
            Lang::Fr => "French",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_codes_round_trip() {
        let codes = ["en", "ru", "be", "pl", "de", "es", "el", "it", "fr"];
        for (lang, code) in Lang::ALL.into_iter().zip(codes) {
            let json = serde_json::to_string(&lang).unwrap();
            assert_eq!(json, format!("\"{code}\""));
            assert_eq!(serde_json::from_str::<Lang>(&json).unwrap(), lang);
        }
        assert!(serde_json::from_str::<Lang>("\"xx\"").is_err());
    }

    #[test]
    fn names_are_english() {
        assert_eq!(Lang::Be.name(), "Belarusian");
        assert_eq!(Lang::El.name(), "Greek");
    }
}
