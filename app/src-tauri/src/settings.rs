//! App settings: API key, hotkey, translator prompts. Stored with
//! tauri-plugin-store in the app data directory (never in `~/Hutarka`).
//! Starting values come from `resources/default-settings.json`, not code.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use hutarka_llm::{Effort, Request};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use crate::translator::{Direction, Kind, Lang, render};

const DEFAULTS_JSON: &str = include_str!("../resources/default-settings.json");
const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "settings";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSpec {
    pub system: String,
    pub model: String,
    pub effort: Effort,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "PartialPrompts")]
pub struct Prompts {
    pub translate: BTreeMap<Direction, PromptSpec>,
    pub word: PromptSpec,
    pub explain: PromptSpec,
}

/// Stored prompts may lack entries (older file, hand edit): the missing
/// ones take the defaults.
#[derive(Deserialize)]
struct PartialPrompts {
    #[serde(default)]
    translate: BTreeMap<Direction, PromptSpec>,
    word: Option<PromptSpec>,
    explain: Option<PromptSpec>,
}

impl From<PartialPrompts> for Prompts {
    fn from(p: PartialPrompts) -> Self {
        let base = &defaults().prompts;
        let mut translate = base.translate.clone();
        translate.extend(p.translate);
        Prompts {
            translate,
            word: p.word.unwrap_or_else(|| base.word.clone()),
            explain: p.explain.unwrap_or_else(|| base.explain.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub api_key: String,
    pub hotkey: String,
    pub english_target: Lang,
    pub explain: bool,
    pub prompts: Prompts,
}

impl Default for Settings {
    fn default() -> Self {
        defaults().clone()
    }
}

/// The default file, read with every field required.
#[derive(Deserialize)]
struct DefaultFile {
    api_key: String,
    hotkey: String,
    english_target: Lang,
    explain: bool,
    prompts: DefaultPrompts,
}

#[derive(Deserialize)]
struct DefaultPrompts {
    translate: BTreeMap<Direction, PromptSpec>,
    word: PromptSpec,
    explain: PromptSpec,
}

/// Settings as shipped. The embedded file is checked by the tests, so a
/// broken one is a build bug.
pub fn defaults() -> &'static Settings {
    static DEFAULTS: OnceLock<Settings> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        let f: DefaultFile =
            serde_json::from_str(DEFAULTS_JSON).expect("resources/default-settings.json is valid");
        Settings {
            api_key: f.api_key,
            hotkey: f.hotkey,
            english_target: f.english_target,
            explain: f.explain,
            prompts: Prompts {
                translate: f.prompts.translate,
                word: f.prompts.word,
                explain: f.prompts.explain,
            },
        }
    })
}

/// One editable prompt: `"translate:en-ru"`, `"word"` or `"explain"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptId {
    Translate(Direction),
    Word,
    Explain,
}

impl Serialize for PromptId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            PromptId::Word => s.serialize_str("word"),
            PromptId::Explain => s.serialize_str("explain"),
            PromptId::Translate(dir) => {
                let code = serde_json::to_value(dir).map_err(serde::ser::Error::custom)?;
                s.serialize_str(&format!("translate:{}", code.as_str().unwrap_or_default()))
            }
        }
    }
}

impl<'de> Deserialize<'de> for PromptId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(d)?;
        match raw.as_str() {
            "word" => Ok(PromptId::Word),
            "explain" => Ok(PromptId::Explain),
            other => {
                let code = other
                    .strip_prefix("translate:")
                    .ok_or_else(|| serde::de::Error::custom(format!("unknown prompt {other}")))?;
                serde_json::from_value(serde_json::Value::from(code))
                    .map(PromptId::Translate)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

impl Prompts {
    pub fn get(&self, id: PromptId) -> &PromptSpec {
        match id {
            // Every direction is present: defaults fill the gaps on load.
            PromptId::Translate(dir) => self
                .translate
                .get(&dir)
                .unwrap_or_else(|| defaults_spec(dir)),
            PromptId::Word => &self.word,
            PromptId::Explain => &self.explain,
        }
    }

    pub fn set(&mut self, id: PromptId, spec: PromptSpec) {
        match id {
            PromptId::Translate(dir) => {
                self.translate.insert(dir, spec);
            }
            PromptId::Word => self.word = spec,
            PromptId::Explain => self.explain = spec,
        }
    }
}

fn defaults_spec(dir: Direction) -> &'static PromptSpec {
    &defaults().prompts.translate[&dir]
}

/// Settings as the Settings window sees them: the key itself never leaves
/// the Rust side.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    pub has_api_key: bool,
    pub hotkey: String,
    pub english_target: Lang,
    pub explain: bool,
    pub prompts: Prompts,
}

impl From<&Settings> for SettingsView {
    fn from(s: &Settings) -> Self {
        SettingsView {
            has_api_key: !s.api_key.trim().is_empty(),
            hotkey: s.hotkey.clone(),
            english_target: s.english_target,
            explain: s.explain,
            prompts: s.prompts.clone(),
        }
    }
}

/// The shipped value of one prompt ("Reset to default").
pub fn default_prompt(id: PromptId) -> PromptSpec {
    defaults().prompts.get(id).clone()
}

/// The request for a selection: prompt chosen by kind, placeholders filled.
pub fn build_request(settings: &Settings, text: &str, dir: Direction, kind: Kind) -> Request {
    let id = match kind {
        Kind::Text => PromptId::Translate(dir),
        Kind::Word => PromptId::Word,
        Kind::Explain => PromptId::Explain,
    };
    let spec = settings.prompts.get(id);
    let mut req = Request::new(spec.model.clone(), render(&spec.system, dir), text);
    req.effort = spec.effort;
    req
}

/// Stored settings, or the defaults when nothing is stored yet or the store
/// cannot be read.
pub fn load(app: &AppHandle) -> Settings {
    let stored = app
        .store(STORE_FILE)
        .map_err(|e| log::error!("cannot open settings store: {e}"))
        .ok()
        .and_then(|store| store.get(STORE_KEY));
    match stored.map(serde_json::from_value::<Settings>) {
        Some(Ok(s)) => s,
        Some(Err(e)) => {
            log::error!("stored settings are invalid, using defaults: {e}");
            Settings::default()
        }
        None => Settings::default(),
    }
}

pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| e.to_string())?;
    let value = serde_json::to_value(settings).map_err(|e| e.to_string())?;
    store.set(STORE_KEY, value);
    store.save().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hutarka_llm::Effort;

    #[test]
    fn defaults_parse() {
        let s = defaults();
        assert_eq!(s.prompts.translate.len(), 4);
        assert_eq!(s.hotkey, "Ctrl+Alt+T");
        assert_eq!(s.english_target, Lang::Ru);
        assert!(!s.explain);
        assert!(s.api_key.is_empty());
        for spec in s.prompts.translate.values() {
            assert_eq!(spec.model, "claude-haiku-5-5");
            assert_eq!(spec.effort, Effort::Low);
            assert!(!spec.system.trim().is_empty());
        }
        assert!(s.prompts.word.system.contains("{to}"));
        assert!(s.prompts.explain.system.contains("{to}"));
    }

    #[test]
    fn default_prompts_ask_for_plain_text() {
        let p = &defaults().prompts;
        for spec in [&p.word, &p.explain] {
            assert!(spec.system.contains("no Markdown"), "{}", spec.system);
        }
    }

    #[test]
    fn missing_fields_take_defaults() {
        let s: Settings = serde_json::from_str(r#"{"api_key":"k"}"#).unwrap();
        assert_eq!(s.api_key, "k");
        assert_eq!(s.prompts, defaults().prompts);
        assert_eq!(s.hotkey, "Ctrl+Alt+T");
    }

    #[test]
    fn missing_direction_takes_default() {
        let s: Settings = serde_json::from_str(
            r#"{"prompts":{"translate":{"en-ru":{"system":"X","model":"m","effort":"high"}}}}"#,
        )
        .unwrap();
        assert_eq!(s.prompts.translate[&Direction::EnRu].system, "X");
        assert_eq!(
            s.prompts.translate[&Direction::PlEn],
            defaults().prompts.translate[&Direction::PlEn]
        );
        assert_eq!(s.prompts.word, defaults().prompts.word);
    }

    #[test]
    fn prompt_id_serde() {
        assert_eq!(
            serde_json::from_str::<PromptId>("\"translate:pl-en\"").unwrap(),
            PromptId::Translate(Direction::PlEn)
        );
        assert_eq!(
            serde_json::to_string(&PromptId::Translate(Direction::EnRu)).unwrap(),
            "\"translate:en-ru\""
        );
        assert_eq!(
            serde_json::to_string(&PromptId::Explain).unwrap(),
            "\"explain\""
        );
        assert!(serde_json::from_str::<PromptId>("\"translate:xx\"").is_err());
    }

    #[test]
    fn effort_serde_lowercase() {
        assert_eq!(
            serde_json::to_string(&Effort::Medium).unwrap(),
            "\"medium\""
        );
        assert_eq!(
            serde_json::from_str::<Effort>("\"high\"").unwrap(),
            Effort::High
        );
    }

    #[test]
    fn build_request_picks_prompt_by_kind() {
        let mut s = defaults().clone();
        s.prompts.word = PromptSpec {
            system: "Word from {from} to {to}".into(),
            model: "claude-sonnet-5-5".into(),
            effort: Effort::Medium,
        };
        let req = build_request(&s, "leverage ", Direction::EnPl, Kind::Word);
        assert_eq!(req.system, "Word from English to Polish");
        assert_eq!(req.model, "claude-sonnet-5-5");
        assert_eq!(req.effort, Effort::Medium);
        assert_eq!(req.text, "leverage ");

        let text = build_request(&s, "t", Direction::RuEn, Kind::Text);
        assert_eq!(text.system, s.prompts.translate[&Direction::RuEn].system);
        let explain = build_request(&s, "t", Direction::RuEn, Kind::Explain);
        assert!(explain.system.contains("English"));
        assert!(!explain.system.contains("{to}"));
    }

    #[test]
    fn view_hides_key() {
        let mut s = defaults().clone();
        s.api_key = "sk-secret".into();
        let view = SettingsView::from(&s);
        assert!(view.has_api_key);
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("sk-secret"));
        assert!(!json.contains("\"api_key\""));
        assert_eq!(view.hotkey, "Ctrl+Alt+T");
        assert!(!SettingsView::from(defaults()).has_api_key);
    }

    #[test]
    fn reset_restores_default() {
        let mut s = defaults().clone();
        s.prompts.set(
            PromptId::Explain,
            PromptSpec {
                system: "seven sentences".into(),
                model: "m".into(),
                effort: Effort::High,
            },
        );
        assert_eq!(s.prompts.explain.system, "seven sentences");
        s.prompts
            .set(PromptId::Explain, default_prompt(PromptId::Explain));
        assert_eq!(s.prompts.explain, defaults().prompts.explain);
    }

    #[test]
    fn default_prompt_matches_defaults() {
        assert_eq!(
            default_prompt(PromptId::Explain),
            defaults().prompts.explain
        );
        assert_eq!(
            default_prompt(PromptId::Translate(Direction::EnPl)),
            defaults().prompts.translate[&Direction::EnPl]
        );
    }
}
