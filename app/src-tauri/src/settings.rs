//! App settings: API key, hotkey, translation targets and prompts. Stored
//! in `settings.json` in the app data directory (never in `~/Hutarka`),
//! written by Hutarka itself (see `settings_file`), never sent to a webview.
//! Starting values come from `resources/default-settings.json`, not code.

use std::sync::OnceLock;

use hutarka_llm::{Effort, Request};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use tauri::{AppHandle, Manager};

use crate::settings_file::{self, Loaded};
use crate::translator::{Kind, Lang, render};

const DEFAULTS_JSON: &str = include_str!("../resources/default-settings.json");
const STORE_FILE: &str = "settings.json";
const STORE_KEY: &str = "settings";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptSpec {
    pub system: String,
    pub model: String,
    pub effort: Effort,
}

/// One target-language card: what to translate into, with which prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Target {
    pub lang: Lang,
    /// Every lookup starts with the default card; exactly one has it.
    pub default: bool,
    pub prompt: PromptSpec,
}

/// The prompts shared by every target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Prompts {
    pub word: PromptSpec,
    pub explain: PromptSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub api_key: String,
    pub hotkey: String,
    pub explain: bool,
    /// Ordered as in Settings and in the popup menu. Never empty, no language
    /// twice, exactly one default (see `normalize`).
    pub targets: Vec<Target>,
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
    explain: bool,
    translate_template: PromptSpec,
    /// Earlier shipped templates: a stored card still carrying one of them
    /// unedited gets the current template on load.
    previous_translate_templates: Vec<String>,
    targets: Vec<DefaultTarget>,
    prompts: Prompts,
    /// Earlier shipped Word and Explain prompts, replaced on load when stored
    /// unedited.
    previous_prompts: PreviousPrompts,
}

#[derive(Deserialize)]
struct PreviousPrompts {
    word: Vec<String>,
    explain: Vec<String>,
}

#[derive(Deserialize)]
struct DefaultTarget {
    lang: Lang,
    default: bool,
}

struct Shipped {
    settings: Settings,
    template: PromptSpec,
    previous_templates: Vec<String>,
    previous_word: Vec<String>,
    previous_explain: Vec<String>,
}

/// The embedded file is checked by the tests, so a broken one is a build bug.
fn shipped() -> &'static Shipped {
    static SHIPPED: OnceLock<Shipped> = OnceLock::new();
    SHIPPED.get_or_init(|| {
        let f: DefaultFile =
            serde_json::from_str(DEFAULTS_JSON).expect("resources/default-settings.json is valid");
        let template = f.translate_template;
        let targets = f
            .targets
            .into_iter()
            .map(|t| Target {
                lang: t.lang,
                default: t.default,
                prompt: template_for(&template, t.lang),
            })
            .collect();
        Shipped {
            settings: Settings {
                api_key: f.api_key,
                hotkey: f.hotkey,
                explain: f.explain,
                targets,
                prompts: f.prompts,
            },
            template,
            previous_templates: f.previous_translate_templates,
            previous_word: f.previous_prompts.word,
            previous_explain: f.previous_prompts.explain,
        }
    })
}

/// Settings as shipped.
pub fn defaults() -> &'static Settings {
    &shipped().settings
}

fn template_for(template: &PromptSpec, lang: Lang) -> PromptSpec {
    PromptSpec {
        system: render(&template.system, lang),
        ..template.clone()
    }
}

/// The shipped translation prompt for `lang`.
fn translate_default(lang: Lang) -> PromptSpec {
    template_for(&shipped().template, lang)
}

/// Makes a target list valid: a repeated language keeps its first card, the
/// first card marked default keeps the mark (the first card gets it when
/// none has it), and an empty list becomes the shipped cards.
pub fn normalize(targets: Vec<Target>) -> Vec<Target> {
    let mut out: Vec<Target> = Vec::with_capacity(targets.len());
    for t in targets {
        if !out.iter().any(|o| o.lang == t.lang) {
            out.push(t);
        }
    }
    if out.is_empty() {
        return defaults().targets.clone();
    }
    let first_default = out.iter().position(|t| t.default).unwrap_or(0);
    for (i, t) in out.iter_mut().enumerate() {
        t.default = i == first_default;
    }
    out
}

/// One editable prompt: `"translate:ru"`, `"word"` or `"explain"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptId {
    Translate(Lang),
    Word,
    Explain,
}

impl Serialize for PromptId {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            PromptId::Word => s.serialize_str("word"),
            PromptId::Explain => s.serialize_str("explain"),
            PromptId::Translate(lang) => {
                let code = serde_json::to_value(lang).map_err(serde::ser::Error::custom)?;
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
                serde_json::from_value(Value::from(code))
                    .map(PromptId::Translate)
                    .map_err(serde::de::Error::custom)
            }
        }
    }
}

impl Settings {
    /// The language every lookup starts with.
    pub fn default_target(&self) -> Lang {
        self.targets
            .iter()
            .find(|t| t.default)
            .or_else(|| self.targets.first())
            .map_or(Lang::En, |t| t.lang)
    }

    /// The prompt to use. A target with no card (removed while the popup
    /// still shows it) gets the shipped translation prompt.
    pub fn prompt(&self, id: PromptId) -> PromptSpec {
        match id {
            PromptId::Translate(lang) => self
                .targets
                .iter()
                .find(|t| t.lang == lang)
                .map_or_else(|| translate_default(lang), |t| t.prompt.clone()),
            PromptId::Word => self.prompts.word.clone(),
            PromptId::Explain => self.prompts.explain.clone(),
        }
    }

    /// Ignored for a target with no card.
    pub fn set_prompt(&mut self, id: PromptId, spec: PromptSpec) {
        match id {
            PromptId::Translate(lang) => {
                if let Some(t) = self.targets.iter_mut().find(|t| t.lang == lang) {
                    t.prompt = spec;
                }
            }
            PromptId::Word => self.prompts.word = spec,
            PromptId::Explain => self.prompts.explain = spec,
        }
    }

    /// Adds a card at the end with the shipped prompt.
    pub fn add_target(&mut self, lang: Lang) -> Result<(), String> {
        if self.targets.iter().any(|t| t.lang == lang) {
            return Err(format!("{} is already in the list", lang.name()));
        }
        self.targets.push(Target {
            lang,
            default: false,
            prompt: translate_default(lang),
        });
        Ok(())
    }

    /// The default card cannot be removed, so the list never ends up empty.
    pub fn remove_target(&mut self, lang: Lang) -> Result<(), String> {
        let i = self
            .targets
            .iter()
            .position(|t| t.lang == lang)
            .ok_or_else(|| format!("{} is not in the list", lang.name()))?;
        if self.targets[i].default {
            return Err("Make another language the default first".into());
        }
        self.targets.remove(i);
        Ok(())
    }

    pub fn set_default_target(&mut self, lang: Lang) -> Result<(), String> {
        if !self.targets.iter().any(|t| t.lang == lang) {
            return Err(format!("{} is not in the list", lang.name()));
        }
        for t in &mut self.targets {
            t.default = t.lang == lang;
        }
        Ok(())
    }
}

/// Settings as the Settings window sees them: the key itself never leaves
/// the Rust side.
#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    pub has_api_key: bool,
    /// The key's last six characters, to tell which key is saved; None for
    /// no key or a key too short to show part of it safely.
    pub api_key_end: Option<String>,
    pub hotkey: String,
    pub explain: bool,
    pub targets: Vec<Target>,
    pub prompts: Prompts,
    /// Every language a card can have, in list order.
    pub languages: Vec<Lang>,
}

/// Last six characters of a key of at least 20 (real keys are ~100).
fn key_end(key: &str) -> Option<String> {
    let chars: Vec<char> = key.trim().chars().collect();
    (chars.len() >= 20).then(|| chars[chars.len() - 6..].iter().collect())
}

impl From<&Settings> for SettingsView {
    fn from(s: &Settings) -> Self {
        SettingsView {
            has_api_key: !s.api_key.trim().is_empty(),
            api_key_end: key_end(&s.api_key),
            hotkey: s.hotkey.clone(),
            explain: s.explain,
            targets: s.targets.clone(),
            prompts: s.prompts.clone(),
            languages: Lang::ALL.to_vec(),
        }
    }
}

/// The shipped value of one prompt ("Reset to default").
pub fn default_prompt(id: PromptId) -> PromptSpec {
    match id {
        PromptId::Translate(lang) => translate_default(lang),
        PromptId::Word => defaults().prompts.word.clone(),
        PromptId::Explain => defaults().prompts.explain.clone(),
    }
}

/// The request for a lookup: prompt chosen by kind, placeholders filled.
pub fn build_request(settings: &Settings, text: &str, to: Lang, kind: Kind) -> Request {
    let id = match kind {
        Kind::Text => PromptId::Translate(to),
        Kind::Word => PromptId::Word,
        Kind::Explain => PromptId::Explain,
    };
    let spec = settings.prompt(id);
    let mut req = Request::new(spec.model.clone(), render(&spec.system, to), text);
    req.effort = spec.effort;
    req
}

fn settings_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|dir| dir.join(STORE_FILE))
        .map_err(|e| format!("no app data folder: {e}"))
}

/// Stored settings, or the defaults when nothing is stored yet. A file that
/// cannot be read or parsed is moved aside (`settings.json.broken-<time>`) so
/// the key and prompts in it can still be recovered by hand, then defaults.
pub fn load(app: &AppHandle) -> Settings {
    let path = match settings_path(app) {
        Ok(path) => path,
        Err(e) => {
            log::error!("{e}");
            return Settings::default();
        }
    };
    let stored = match settings_file::read(&path) {
        Loaded::Missing => return Settings::default(),
        Loaded::Unreadable(e) => {
            let stamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
            match settings_file::set_aside(&path, &stamp) {
                Ok(kept) => log::error!(
                    "cannot read settings ({e}); kept as {}, defaults used",
                    kept.display()
                ),
                Err(move_err) => log::error!(
                    "cannot read settings ({e}) nor move the file aside ({move_err}); defaults used"
                ),
            }
            return Settings::default();
        }
        Loaded::Stored(value) => value,
    };
    let Some(stored) = stored.get(STORE_KEY).cloned() else {
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

/// Reads stored settings field by field: an invalid field (hand edit) takes
/// its default and is named in the returned list, while the rest (the key,
/// edited prompts) is kept, so the next save does not wipe it. Fields of
/// older versions (`english_target`, `prompts.translate`) are ignored, and
/// prompts still carrying an earlier shipped text get the current one.
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
    take(obj.get("explain"), "explain", &mut s.explain, &mut bad);
    match obj.get("targets") {
        None => {}
        Some(Value::Array(cards)) => {
            s.targets = cards
                .iter()
                .enumerate()
                .filter_map(|(i, card)| {
                    let parsed = stored_target(card);
                    if parsed.is_none() {
                        bad.push(format!("targets[{i}]"));
                    }
                    parsed
                })
                .collect();
        }
        Some(_) => bad.push("targets".into()),
    }
    s.targets = normalize(std::mem::take(&mut s.targets));
    match obj.get("prompts") {
        None => {}
        Some(Value::Object(prompts)) => {
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
    let base = &defaults().prompts;
    if shipped().previous_word.contains(&s.prompts.word.system) {
        s.prompts.word.system = base.word.system.clone();
    }
    if shipped()
        .previous_explain
        .contains(&s.prompts.explain.system)
    {
        s.prompts.explain.system = base.explain.system.clone();
    }
    (s, bad)
}

/// One stored card; a card without a prompt gets the shipped one, so a
/// broken card costs only itself. An unedited earlier shipped prompt is
/// replaced by the current one (model and effort are kept).
fn stored_target(card: &Value) -> Option<Target> {
    #[derive(Deserialize)]
    struct Stored {
        lang: Lang,
        #[serde(default)]
        default: bool,
        prompt: Option<PromptSpec>,
    }
    let c: Stored = serde_json::from_value(card.clone()).ok()?;
    let mut prompt = c.prompt.unwrap_or_else(|| translate_default(c.lang));
    let stale = shipped()
        .previous_templates
        .iter()
        .any(|old| render(old, c.lang) == prompt.system);
    if stale {
        prompt.system = translate_default(c.lang).system;
    }
    Some(Target {
        lang: c.lang,
        default: c.default,
        prompt,
    })
}

/// Writes the settings; done only when the file is safely replaced.
pub fn save(app: &AppHandle, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app)?;
    let value = serde_json::json!({ STORE_KEY: settings });
    settings_file::write(&path, &value).map_err(|e| format!("cannot save settings: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn card(lang: Lang, default: bool) -> Target {
        Target {
            lang,
            default,
            prompt: translate_default(lang),
        }
    }

    fn langs(targets: &[Target]) -> Vec<(Lang, bool)> {
        targets.iter().map(|t| (t.lang, t.default)).collect()
    }

    #[test]
    fn defaults_have_three_targets_english_default() {
        let s = defaults();
        assert_eq!(
            langs(&s.targets),
            vec![(Lang::En, true), (Lang::Ru, false), (Lang::Pl, false)]
        );
        assert_eq!(s.default_target(), Lang::En);
        assert_eq!(s.hotkey, "Ctrl+Alt+T");
        assert!(!s.explain);
        assert!(s.api_key.is_empty());
        for t in &s.targets {
            assert_eq!(t.prompt.model, "claude-haiku-5-5");
            assert_eq!(t.prompt.effort, Effort::Low);
            assert!(t.prompt.system.contains(t.lang.name()));
            assert!(!t.prompt.system.contains("{to}"));
        }
        assert!(s.prompts.word.system.contains("{to}"));
        assert!(!s.prompts.word.system.contains("{from}"));
        assert!(!s.prompts.explain.system.contains("{from}"));
    }

    #[test]
    fn default_prompts_ask_for_plain_text() {
        let p = &defaults().prompts;
        for spec in [&p.word, &p.explain] {
            assert!(spec.system.contains("no Markdown"), "{}", spec.system);
        }
    }

    #[test]
    fn normalize_drops_duplicates_and_keeps_one_default() {
        let out = normalize(vec![
            card(Lang::Ru, false),
            card(Lang::De, true),
            card(Lang::Ru, true),
            card(Lang::Fr, true),
        ]);
        assert_eq!(
            langs(&out),
            vec![(Lang::Ru, false), (Lang::De, true), (Lang::Fr, false)]
        );
    }

    #[test]
    fn normalize_without_default_marks_first() {
        let out = normalize(vec![card(Lang::De, false), card(Lang::Fr, false)]);
        assert_eq!(langs(&out), vec![(Lang::De, true), (Lang::Fr, false)]);
    }

    #[test]
    fn normalize_empty_gives_shipped() {
        assert_eq!(normalize(vec![]), defaults().targets);
    }

    #[test]
    fn old_file_loads_without_bad_fields() {
        let stored = json!({
            "api_key": "sk-kept",
            "hotkey": "Ctrl+Cmd+T",
            "english_target": "pl",
            "explain": true,
            "prompts": {
                "translate": {"en-ru": {"system": "X", "model": "m", "effort": "low"}},
                "word": {"system": "my word prompt {from}", "model": "m", "effort": "high"},
                "explain": defaults().prompts.explain,
            }
        });
        let (s, bad) = from_stored(stored);
        assert!(bad.is_empty(), "{bad:?}");
        assert_eq!(s.api_key, "sk-kept");
        assert_eq!(s.hotkey, "Ctrl+Cmd+T");
        assert!(s.explain);
        assert_eq!(s.prompts.word.system, "my word prompt {from}");
        assert_eq!(s.targets, defaults().targets);
    }

    #[test]
    fn invalid_targets_take_defaults() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["targets"] = 5.into();
        stored["api_key"] = "sk-kept".into();
        let (s, bad) = from_stored(stored);
        assert_eq!(bad, vec!["targets".to_string()]);
        assert_eq!(s.targets, defaults().targets);
        assert_eq!(s.api_key, "sk-kept");
    }

    #[test]
    fn stored_targets_are_normalized() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["targets"] =
            serde_json::to_value(vec![card(Lang::De, false), card(Lang::De, false)])
                .expect("value");
        let (s, bad) = from_stored(stored);
        assert!(bad.is_empty());
        assert_eq!(langs(&s.targets), vec![(Lang::De, true)]);
    }

    #[test]
    fn one_bad_card_keeps_the_other_cards() {
        let mut ru = card(Lang::Ru, false);
        ru.prompt.system = "my Russian prompt".into();
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["targets"] = json!([
            card(Lang::En, true),
            {"lang": "xx", "default": false, "prompt": defaults().prompts.word},
            ru,
            {"lang": "de", "default": false},
        ]);
        let (s, bad) = from_stored(stored);
        assert_eq!(bad, vec!["targets[1]".to_string()]);
        assert_eq!(
            langs(&s.targets),
            vec![(Lang::En, true), (Lang::Ru, false), (Lang::De, false)]
        );
        assert_eq!(s.targets[1].prompt.system, "my Russian prompt");
        assert_eq!(s.targets[2].prompt, translate_default(Lang::De));
    }

    #[test]
    fn card_with_an_old_shipped_prompt_gets_the_new_one() {
        let old = shipped().previous_templates[0].replace("{to}", "English");
        let mut stale = card(Lang::En, true);
        stale.prompt.system = old;
        let mut edited = card(Lang::Ru, false);
        edited.prompt.system = "my Russian prompt".into();
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["targets"] = serde_json::to_value(vec![stale, edited]).expect("value");
        let (s, bad) = from_stored(stored);
        assert!(bad.is_empty());
        assert_eq!(s.targets[0].prompt, translate_default(Lang::En));
        assert_eq!(s.targets[1].prompt.system, "my Russian prompt");
    }

    #[test]
    fn old_shipped_word_and_explain_prompts_get_the_new_ones() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["prompts"]["word"]["system"] = shipped().previous_word[0].clone().into();
        stored["prompts"]["word"]["model"] = "claude-sonnet-5-5".into();
        stored["prompts"]["explain"]["system"] = "my explain".into();
        let (s, bad) = from_stored(stored);
        assert!(bad.is_empty());
        assert_eq!(s.prompts.word.system, defaults().prompts.word.system);
        assert_eq!(s.prompts.word.model, "claude-sonnet-5-5");
        assert_eq!(s.prompts.explain.system, "my explain");
    }

    #[test]
    fn one_invalid_field_keeps_the_others() {
        let mut stored = serde_json::to_value(defaults()).expect("value");
        stored["api_key"] = "sk-kept".into();
        stored["explain"] = "x".into();
        stored["prompts"]["word"]["system"] = "my word prompt".into();
        let (s, bad) = from_stored(stored);
        assert_eq!(s.api_key, "sk-kept");
        assert_eq!(s.prompts.word.system, "my word prompt");
        assert_eq!(bad, vec!["explain".to_string()]);
    }

    #[test]
    fn valid_stored_settings_report_nothing() {
        let (s, bad) = from_stored(serde_json::to_value(defaults()).expect("value"));
        assert_eq!(&s, defaults());
        assert!(bad.is_empty());
    }

    #[test]
    fn add_target_refuses_repeat() {
        let mut s = defaults().clone();
        s.add_target(Lang::De).unwrap();
        assert_eq!(s.targets.last().unwrap().lang, Lang::De);
        assert!(!s.targets.last().unwrap().default);
        assert!(s.targets.last().unwrap().prompt.system.contains("German"));
        assert_eq!(
            s.add_target(Lang::Ru),
            Err("Russian is already in the list".into())
        );
    }

    #[test]
    fn remove_target_refuses_default() {
        let mut s = defaults().clone();
        assert_eq!(
            s.remove_target(Lang::En),
            Err("Make another language the default first".into())
        );
        s.remove_target(Lang::Pl).unwrap();
        assert_eq!(langs(&s.targets), vec![(Lang::En, true), (Lang::Ru, false)]);
        assert!(s.remove_target(Lang::Pl).is_err());
    }

    #[test]
    fn set_default_moves_mark() {
        let mut s = defaults().clone();
        s.set_default_target(Lang::Ru).unwrap();
        assert_eq!(s.default_target(), Lang::Ru);
        assert_eq!(
            langs(&s.targets),
            vec![(Lang::En, false), (Lang::Ru, true), (Lang::Pl, false)]
        );
        assert!(s.set_default_target(Lang::Fr).is_err());
    }

    #[test]
    fn prompt_for_missing_target_uses_template() {
        let s = defaults().clone();
        let spec = s.prompt(PromptId::Translate(Lang::El));
        assert_eq!(spec, default_prompt(PromptId::Translate(Lang::El)));
        assert!(spec.system.contains("into Greek"));
    }

    #[test]
    fn build_request_text_uses_card_prompt() {
        let mut s = defaults().clone();
        let mine = PromptSpec {
            system: "Into {to}, IT field".into(),
            model: "claude-sonnet-5-5".into(),
            effort: Effort::Medium,
        };
        s.set_prompt(PromptId::Translate(Lang::Ru), mine);
        let req = build_request(&s, "text ", Lang::Ru, Kind::Text);
        assert_eq!(req.system, "Into Russian, IT field");
        assert_eq!(req.model, "claude-sonnet-5-5");
        assert_eq!(req.effort, Effort::Medium);
        assert_eq!(req.text, "text ");
    }

    #[test]
    fn word_prompt_renders_to() {
        let s = defaults().clone();
        let req = build_request(&s, "leverage", Lang::Pl, Kind::Word);
        assert!(req.system.contains("Answer in Polish"));
        assert!(!req.system.contains("{to}"));
        let explain = build_request(&s, "t", Lang::De, Kind::Explain);
        assert!(explain.system.contains("German"));
    }

    #[test]
    fn prompt_id_serde() {
        assert_eq!(
            serde_json::from_str::<PromptId>("\"translate:pl\"").unwrap(),
            PromptId::Translate(Lang::Pl)
        );
        assert_eq!(
            serde_json::to_string(&PromptId::Translate(Lang::El)).unwrap(),
            "\"translate:el\""
        );
        assert_eq!(
            serde_json::to_string(&PromptId::Explain).unwrap(),
            "\"explain\""
        );
        assert!(serde_json::from_str::<PromptId>("\"translate:xx\"").is_err());
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
        assert!(!SettingsView::from(defaults()).has_api_key);
    }

    #[test]
    fn view_shows_only_the_last_six_characters_of_the_key() {
        let mut s = defaults().clone();
        s.api_key = "sk-ant-api03-secretvalue-AbC123".into();
        let view = SettingsView::from(&s);
        assert_eq!(view.api_key_end.as_deref(), Some("AbC123"));
        let json = serde_json::to_string(&view).unwrap();
        assert!(!json.contains("secretvalue"));
        // A short key would be mostly revealed: no hint.
        s.api_key = "short-key".into();
        assert_eq!(SettingsView::from(&s).api_key_end, None);
        assert_eq!(SettingsView::from(defaults()).api_key_end, None);
    }

    #[test]
    fn reset_restores_default() {
        let mut s = defaults().clone();
        s.set_prompt(
            PromptId::Translate(Lang::Ru),
            PromptSpec {
                system: "mine".into(),
                model: "m".into(),
                effort: Effort::High,
            },
        );
        s.set_prompt(
            PromptId::Translate(Lang::Ru),
            default_prompt(PromptId::Translate(Lang::Ru)),
        );
        assert_eq!(s, *defaults());
    }
}
