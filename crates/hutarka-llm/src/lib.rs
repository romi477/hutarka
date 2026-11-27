//! Transport to LLM providers. Sends exactly the prompt, model and text it is
//! given; contains no prompts of its own.

mod client;
mod error;
mod request;
mod sse;

pub use client::Client;
pub use error::Error;
pub use request::{DEFAULT_MODEL, Effort, Request, build_body, build_stream_body, parse_response};
