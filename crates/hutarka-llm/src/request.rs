use serde_json::{Value, json};

use crate::Error;

/// Model used when the caller does not choose one.
pub const DEFAULT_MODEL: &str = "claude-haiku-5-5";

/// How much the model thinks before answering: trades speed and cost for depth.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    #[default]
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn as_str(&self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
        }
    }
}

/// One text-in, text-out call: a system prompt and the user's text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub model: String,
    pub system: String,
    pub text: String,
    pub effort: Effort,
    pub max_tokens: u32,
}

impl Request {
    pub fn new(
        model: impl Into<String>,
        system: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Request {
            model: model.into(),
            system: system.into(),
            text: text.into(),
            effort: Effort::Low,
            max_tokens: 16000,
        }
    }
}

/// The Messages API request body. Sends no sampling parameters and no
/// `thinking` field: effort is the one control that works on every model.
pub fn build_body(req: &Request) -> Value {
    json!({
        "model": req.model,
        "max_tokens": req.max_tokens,
        "system": req.system,
        "messages": [{"role": "user", "content": req.text}],
        "output_config": {"effort": req.effort.as_str()}
    })
}

/// The body of a streamed request: `build_body` plus `"stream": true`.
pub fn build_stream_body(req: &Request) -> Value {
    let mut body = build_body(req);
    body["stream"] = Value::Bool(true);
    body
}

/// Only a natural end counts as a complete answer.
pub(crate) fn check_stop(reason: Option<&str>) -> Result<(), Error> {
    match reason {
        Some("end_turn" | "stop_sequence") => Ok(()),
        Some("refusal") => Err(Error::Refusal),
        Some("max_tokens") => Err(Error::Truncated),
        Some(other) => Err(Error::Stopped(other.to_string())),
        None => Err(Error::Stopped("none".into())),
    }
}

/// The answer's text: every `text` block joined, other blocks skipped.
/// Only a normal finish (`end_turn`, `stop_sequence`) counts: a refusal or any
/// other stop reason is an error, never a partial result.
pub fn parse_response(body: &Value) -> Result<String, Error> {
    let content = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(|| Error::BadResponse("no content array".into()))?;
    check_stop(body.get("stop_reason").and_then(Value::as_str))?;
    let text: String = content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    if text.is_empty() {
        return Err(Error::Empty);
    }
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use serde_json::json;

    #[test]
    fn build_body_has_exact_shape() {
        let req = Request::new("claude-haiku-5-5", "SYS", "привет");
        assert_eq!(
            build_body(&req),
            json!({
                "model": "claude-haiku-5-5",
                "max_tokens": 16000,
                "system": "SYS",
                "messages": [{"role": "user", "content": "привет"}],
                "output_config": {"effort": "low"}
            })
        );
    }

    #[test]
    fn build_body_never_sends_sampling_or_thinking() {
        let body = build_body(&Request::new("m", "s", "t"));
        let obj = body.as_object().unwrap();
        for key in ["temperature", "top_p", "top_k", "thinking"] {
            assert!(!obj.contains_key(key), "unexpected key {key}");
        }
    }

    #[test]
    fn parse_joins_text_blocks_and_skips_others() {
        let body = json!({
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": "Hello"},
                {"type": "text", "text": " world"}
            ],
            "stop_reason": "end_turn"
        });
        assert_eq!(parse_response(&body).unwrap(), "Hello world");
    }

    #[test]
    fn parse_rejects_refusal() {
        let body = json!({"content": [], "stop_reason": "refusal"});
        assert!(matches!(parse_response(&body), Err(Error::Refusal)));
    }

    #[test]
    fn parse_rejects_max_tokens() {
        let body = json!({
            "content": [{"type": "text", "text": "partial"}],
            "stop_reason": "max_tokens"
        });
        assert!(matches!(parse_response(&body), Err(Error::Truncated)));
    }

    #[test]
    fn parse_rejects_no_text() {
        let body = json!({"content": [], "stop_reason": "end_turn"});
        assert!(matches!(parse_response(&body), Err(Error::Empty)));
    }

    #[test]
    fn parse_rejects_unknown_or_missing_stop_reason() {
        let text = json!([{"type": "text", "text": "partial"}]);
        let unknown = json!({"content": text, "stop_reason": "model_context_window_exceeded"});
        assert_eq!(
            parse_response(&unknown),
            Err(Error::Stopped("model_context_window_exceeded".into()))
        );
        let missing = json!({"content": text});
        assert_eq!(parse_response(&missing), Err(Error::Stopped("none".into())));
        let stop_sequence = json!({"content": text, "stop_reason": "stop_sequence"});
        assert_eq!(parse_response(&stop_sequence), Ok("partial".into()));
    }

    #[test]
    fn parse_rejects_missing_content() {
        assert!(matches!(
            parse_response(&json!({})),
            Err(Error::BadResponse(_))
        ));
    }
}
