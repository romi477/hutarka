//! App settings: API key, hotkey, translator prompts. Stored with
//! tauri-plugin-store in the app data directory (never in `~/Hutarka`).
//! Starting values come from `resources/default-settings.json`, not code.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use hutarka_llm::{Effort, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
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
    translate: DefaultTranslate,
    word: PromptSpec,
    explain: PromptSpec,
}

/// All four directions are required, so every direction has a default.
#[derive(Deserialize)]
struct DefaultTranslate {
    #[serde(rename = "en-ru")]
    en_ru: PromptSpec,
    #[serde(rename = "en-pl")]
    en_pl: PromptSpec,
    #[serde(rename = "pl-en")]
    pl_en: PromptSpec,
    #[serde(rename = "ru-en")]
    ru_en: PromptSpec,
}

impl DefaultTranslate {
    fn spec(&self, dir: Direction) -> &PromptSpec {
        match dir {
            Direction::EnRu => &self.en_ru,
            Direction::EnPl => &self.en_pl,
            Direction::PlEn => &self.pl_en,
            Direction::RuEn => &self.ru_en,
        }
    }
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
                translate: [
                    Direction::EnRu,
                    Direction::EnPl,
                    Direction::PlEn,
                    Direction::RuEn,
                ]
                .into_iter()
                .map(|d| (d, f.prompts.translate.spec(d).clone()))
                .collect(),
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

/// Never panics: `defaults()` fills the map from every `Direction` variant.
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
    let Some(stored) = stored else {
        return Settings::default();
    };
    let (settings, bad) = from_stored(stored);
    if !bad.is_empty() {
        log::error!(
            "invalid stored settings, defaults used for: {}",
            bad.join(", ")
        );
    }
    settings
}

/// Reads stored settings field by field: an invalid field (hand edit, older
/// format) takes its default and is named in the returned list, while the
/// rest (the key, edited prompts) is kept, so the next save does not wipe it.
fn from_stored(stored: Value) -> (Settings, Vec<String>) {
    let mut s = Settings::default();
    let mut bad = Vec::new();
    let Some(obj) = stored.as_object() else {
        return (s, vec!["settings".into()]);
    };
    fn take<T: DeserializeOwned>(
        v: Option<&Value>,
        name: &str,
        slot: &mut T,
        bad: &mut Vec<String>,
    ) {
        if let Some(v) = v {
            match serde_json::from_value(v.clone()) {
                Ok(x) => *slot = x,
                Err(_) => bad.push(name.to_string()),
            }
        }
    }
    take(obj.get("api_key"), "api_key", &mut s.api_key, &mut bad);
    take(obj.get("hotkey"), "hotkey", &mut s.hotkey, &mut bad);
    take(
        obj.get("english_target"),
        "english_target",
        &mut s.english_target,
        &mut bad,
    );
    take(obj.get("explain"), "explain", &mut s.explain, &mut bad);
    match obj.get("prompts") {
        None => {}
        Some(Value::Object(prompts)) => {
            if let Some(translate) = prompts.get("translate") {
                match translate.as_object() {
                    Some(map) => {
                        for (code, spec) in map {
                            let name = format!("prompts.translate.{code}");
                            match serde_json::from_value::<Direction>(Value::from(code.as_str())) {
                                Ok(dir) => {
                                    let slot = s
                                        .prompts
                                        .translate
                                        .entry(dir)
                                        .or_insert_with(|| defaults_spec(dir).clone());
                                    take(Some(spec), &name, slot, &mut bad);
                                }
                                Err(_) => bad.push(name),
                            }
                        }
                    }
                    None => bad.push("prompts.translate".into()),
                }
            }
            take(
                prompts.get("word"),
                "prompts.word",
                &mut s.prompts.word,
                &mut bad,
            );
            take(
                prompts.get("explain"),
                "prompts.explain",
                &mut s.prompts.explain,
                &mut bad,
            );
        }
        Some(_) => bad.push("prompts".into()),
    }
    (s, bad)
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

    #[test]
    fn one_invalid_field_keeps_the_others() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["api_key"] = "sk-kept".into();
        stored["english_target"] = "de".into();
        stored["prompts"]["word"]["system"] = "my word prompt".into();
        let (s, bad) = from_stored(stored);
        assert_eq!(s.api_key, "sk-kept");
        assert_eq!(s.english_target, defaults().english_target);
        assert_eq!(s.prompts.word.system, "my word prompt");
        assert_eq!(bad, vec!["english_target".to_string()]);
    }

    #[test]
    fn one_invalid_prompt_keeps_the_other_prompts() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["prompts"]["word"]["system"] = "my word prompt".into();
        stored["prompts"]["explain"]["effort"] = "ultra".into();
        let (s, bad) = from_stored(stored);
        assert_eq!(s.prompts.word.system, "my word prompt");
        assert_eq!(s.prompts.explain, defaults().prompts.explain);
        assert_eq!(bad, vec!["prompts.explain".to_string()]);
    }

    #[test]
    fn valid_stored_settings_report_nothing() {
        let (s, bad) = from_stored(serde_json::to_value(defaults()).expect("value"));
        assert_eq!(&s, defaults());
        assert!(bad.is_empty());
    }
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
