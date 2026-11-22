use clap::{ArgGroup, Parser, ValueEnum};
use hutarka_llm::{DEFAULT_MODEL, Effort};

/// Send text to an LLM with an explicit prompt and print the answer.
///
/// Input: TEXT, or stdin when TEXT is omitted. Key: HUTARKA_API_KEY.
#[derive(Debug, Parser)]
#[command(name = "hutarka", version)]
#[command(group(ArgGroup::new("instruction").required(true).multiple(false).args(["prompt", "to"])))]
pub struct Args {
    /// System prompt, sent exactly as given.
    #[arg(long)]
    pub prompt: Option<String>,

    /// Plain literal translation into LANG (en, ru, be, pl or any language name).
    ///
    /// Sends this fixed prompt and nothing else:
    /// "Translate the text to <LANG>. Output only the translation."
    #[arg(long, value_name = "LANG")]
    pub to: Option<String>,

    /// Model id.
    #[arg(long, default_value = DEFAULT_MODEL)]
    pub model: String,

    /// How much the model thinks before answering.
    #[arg(long, value_enum, default_value_t = EffortArg::Low)]
    pub effort: EffortArg,

    /// Text to process; read from stdin when omitted.
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EffortArg {
    Low,
    Medium,
    High,
}

impl From<EffortArg> for Effort {
    fn from(e: EffortArg) -> Self {
        match e {
            EffortArg::Low => Effort::Low,
            EffortArg::Medium => Effort::Medium,
            EffortArg::High => Effort::High,
        }
    }
}

/// The system prompt for this call: `--prompt` as is, or the `--to` prompt.
pub fn system_prompt(args: &Args) -> String {
    match (&args.prompt, &args.to) {
        (Some(prompt), _) => prompt.clone(),
        (None, Some(lang)) => translate_prompt(lang),
        (None, None) => String::new(),
    }
}

/// The one prompt the CLI contains.
pub fn translate_prompt(lang: &str) -> String {
    format!(
        "Translate the text to {}. Output only the translation.",
        language_name(lang)
    )
}

/// Full name for the short codes the author uses; anything else as given.
pub fn language_name(code: &str) -> String {
    match code.to_ascii_lowercase().as_str() {
        "en" => "English".into(),
        "ru" => "Russian".into(),
        "be" => "Belarusian".into(),
        "pl" => "Polish".into(),
        _ => code.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{CommandFactory, Parser};
    use hutarka_llm::Effort;

    #[test]
    fn to_and_prompt_are_exclusive() {
        assert!(Args::try_parse_from(["hutarka", "--to", "en", "--prompt", "x", "t"]).is_err());
    }

    #[test]
    fn one_instruction_is_required() {
        assert!(Args::try_parse_from(["hutarka", "t"]).is_err());
    }

    #[test]
    fn defaults_are_haiku_and_low() {
        let args = Args::try_parse_from(["hutarka", "--to", "en", "t"]).unwrap();
        assert_eq!(args.model, "claude-haiku-5-5");
        assert_eq!(Effort::from(args.effort), Effort::Low);
    }

    #[test]
    fn translate_prompt_is_exact() {
        assert_eq!(
            translate_prompt("be"),
            "Translate the text to Belarusian. Output only the translation."
        );
        assert_eq!(
            translate_prompt("German"),
            "Translate the text to German. Output only the translation."
        );
        assert_eq!(language_name("EN"), "English");
    }

    #[test]
    fn prompt_is_sent_as_given() {
        let args = Args::try_parse_from(["hutarka", "--prompt", "  Keep\nspacing ", "t"]).unwrap();
        assert_eq!(system_prompt(&args), "  Keep\nspacing ");
    }

    #[test]
    fn help_shows_the_translate_prompt() {
        let help = Args::command().render_long_help().to_string();
        assert!(help.contains("Output only the translation."), "{help}");
    }
}
