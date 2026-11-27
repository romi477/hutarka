/// Everything that can go wrong between sending a request and getting text back.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum Error {
    #[error("HUTARKA_API_KEY is not set")]
    MissingKey,
    #[error("network error: {0}")]
    Network(String),
    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },
    #[error("the model declined the request")]
    Refusal,
    #[error("the answer was cut off (max_tokens)")]
    Truncated,
    #[error("the answer stopped early ({0})")]
    Stopped(String),
    #[error("the model returned no text")]
    Empty,
    #[error("unexpected response: {0}")]
    BadResponse(String),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("cannot create the HTTP client: {0}")]
    Setup(String),
}
