use std::time::Duration;

use serde_json::Value;

use futures_util::StreamExt;

use crate::request::{build_stream_body, check_stop};
use crate::sse::{SseEvent, SseParser};
use crate::{Error, Request, build_body, parse_response};

pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
const API_VERSION: &str = "2026-10-21";

/// Anthropic Messages API client.
pub struct Client {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl Client {
    pub fn new(api_key: String, base_url: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        Client {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    /// Key from `HUTARKA_API_KEY`; base URL from `HUTARKA_BASE_URL`, else the
    /// public API.
    pub fn from_env() -> Result<Self, Error> {
        let raw = std::env::var("HUTARKA_API_KEY").unwrap_or_default();
        let key = raw.trim().to_string();
        if key.is_empty() {
            return Err(Error::MissingKey);
        }
        let base = std::env::var("HUTARKA_BASE_URL")
            .ok()
            .filter(|b| !b.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());
        Ok(Client::new(key, base))
    }

    /// Sends one request and returns the answer's text.
    pub async fn complete(&self, req: &Request) -> Result<String, Error> {
        let response = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&build_body(req))
            .send()
            .await
            .map_err(network_error)?;
        let status = response.status();
        let body = response.text().await.map_err(network_error)?;
        if !status.is_success() {
            return Err(Error::Api {
                status: status.as_u16(),
                message: api_message(status, &body),
            });
        }
        let json: Value =
            serde_json::from_str(&body).map_err(|e| Error::BadResponse(e.to_string()))?;
        parse_response(&json)
    }

    /// Sends one streamed request: `on_text` gets each piece of text as it
    /// arrives; returns the whole text once the answer ends normally.
    pub async fn stream<F: FnMut(&str)>(
        &self,
        req: &Request,
        mut on_text: F,
    ) -> Result<String, Error> {
        let response = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", API_VERSION)
            .json(&build_stream_body(req))
            .send()
            .await
            .map_err(network_error)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.map_err(network_error)?;
            return Err(Error::Api {
                status: status.as_u16(),
                message: api_message(status, &body),
            });
        }
        let mut parser = SseParser::default();
        let mut text = String::new();
        let mut stop: Option<String> = None;
        let mut bytes = response.bytes_stream();
        while let Some(chunk) = bytes.next().await {
            for event in parser.push(&chunk.map_err(network_error)?) {
                match event {
                    SseEvent::TextDelta(piece) => {
                        on_text(&piece);
                        text.push_str(&piece);
                    }
                    SseEvent::Stop(reason) => stop = reason.or(stop),
                    SseEvent::Error(message) => return Err(Error::Stream(message)),
                    SseEvent::Other => {}
                }
            }
        }
        check_stop(stop.as_deref())?;
        if text.is_empty() {
            return Err(Error::Empty);
        }
        Ok(text)
    }
}

/// A network error with its whole cause chain ("... : connection refused"),
/// since reqwest's own message leaves the cause out.
fn network_error(e: reqwest::Error) -> Error {
    let mut msg = if e.is_timeout() {
        "timed out".to_string()
    } else {
        e.to_string()
    };
    let mut source = std::error::Error::source(&e);
    while let Some(cause) = source {
        msg.push_str(": ");
        msg.push_str(&cause.to_string());
        source = cause.source();
    }
    Error::Network(msg)
}

/// One readable line for an API error: `error.message` from a JSON body;
/// otherwise the body collapsed to one line and capped at 200 characters;
/// the HTTP reason ("Bad Gateway") when the body is empty or HTML.
fn api_message(status: reqwest::StatusCode, body: &str) -> String {
    if let Some(message) = serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|v| v.pointer("/error/message")?.as_str().map(str::to_string))
    {
        return message;
    }
    let line = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if line.is_empty() || line.starts_with('<') {
        return status
            .canonical_reason()
            .unwrap_or("unknown error")
            .to_string();
    }
    line.chars().take(200).collect()
}
