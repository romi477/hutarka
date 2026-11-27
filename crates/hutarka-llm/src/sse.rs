//! Parser for the Messages API's server-sent events.

use serde_json::Value;

/// One event of a streamed answer that the client cares about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SseEvent {
    TextDelta(String),
    Stop(Option<String>),
    Error(String),
    Other,
}

/// Turns raw response bytes, in chunks of any size, into events.
#[derive(Debug, Default)]
pub struct SseParser {
    buf: Vec<u8>,
}

impl SseParser {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<SseEvent> {
        // CR only appears in line endings: JSON escapes it inside strings.
        self.buf
            .extend(chunk.iter().copied().filter(|&b| b != b'\r'));
        let mut events = Vec::new();
        while let Some(end) = self.buf.windows(2).position(|w| w == b"\n\n") {
            let block: Vec<u8> = self.buf.drain(..end + 2).collect();
            events.push(parse_block(&String::from_utf8_lossy(&block)));
        }
        events
    }
}

fn parse_block(block: &str) -> SseEvent {
    let data: Vec<&str> = block
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .map(|d| d.strip_prefix(' ').unwrap_or(d))
        .collect();
    let Ok(json) = serde_json::from_str::<Value>(&data.join("\n")) else {
        return SseEvent::Other;
    };
    match json.get("type").and_then(Value::as_str) {
        Some("content_block_delta") => match json.pointer("/delta/text").and_then(Value::as_str) {
            Some(text) if json.pointer("/delta/type") == Some(&Value::from("text_delta")) => {
                SseEvent::TextDelta(text.to_string())
            }
            _ => SseEvent::Other,
        },
        Some("message_delta") => SseEvent::Stop(
            json.pointer("/delta/stop_reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        ),
        Some("error") => SseEvent::Error(
            json.pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
                .to_string(),
        ),
        _ => SseEvent::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(chunks: &[&[u8]]) -> Vec<SseEvent> {
        let mut parser = SseParser::default();
        chunks.iter().flat_map(|c| parser.push(c)).collect()
    }

    #[test]
    fn parses_text_delta() {
        let raw = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"При\"}}\n\n";
        assert_eq!(
            feed(&[raw.as_bytes()]),
            vec![SseEvent::TextDelta("При".into())]
        );
    }

    #[test]
    fn joins_event_split_across_chunks() {
        let raw = "event: content_block_delta\r\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Привет\"}}\r\n\r\n";
        let bytes = raw.as_bytes();
        // Split inside the two-byte "и" so a chunk ends mid-character.
        let cut = raw.find("ри").unwrap() + 3;
        assert_eq!(
            feed(&[&bytes[..cut], &bytes[cut..]]),
            vec![SseEvent::TextDelta("Привет".into())]
        );
    }

    #[test]
    fn message_delta_gives_stop_reason() {
        let raw = "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n";
        assert_eq!(
            feed(&[raw.as_bytes()]),
            vec![SseEvent::Stop(Some("end_turn".into()))]
        );
    }

    #[test]
    fn error_event() {
        let raw = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n";
        assert_eq!(
            feed(&[raw.as_bytes()]),
            vec![SseEvent::Error("Overloaded".into())]
        );
    }

    #[test]
    fn ignores_ping() {
        let raw = "event: ping\ndata: {\"type\": \"ping\"}\n\n";
        assert_eq!(feed(&[raw.as_bytes()]), vec![SseEvent::Other]);
    }
}
