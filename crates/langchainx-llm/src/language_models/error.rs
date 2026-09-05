//! Errors returned by language-model clients.
use async_openai::error::OpenAIError;
#[cfg(feature = "ollama")]
use ollama_rs::error::OllamaError;
use reqwest::Error as ReqwestError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use tokio::time::error::Elapsed;

use crate::{AnthropicError, DeepseekError, QwenError};

#[derive(Error, Debug)]
/// Errors that can occur while preparing, sending, or parsing model requests.
pub enum LLMError {
    #[error("OpenAI error: {0}")]
    /// The OpenAI-compatible client returned an error.
    OpenAIError(#[from] OpenAIError),

    #[error("Anthropic error: {0}")]
    /// The Anthropic API returned an error.
    AnthropicError(#[from] AnthropicError),

    #[error("Qwen error: {0}")]
    /// The Qwen API returned an error.
    QwenError(#[from] QwenError),

    #[error("Deepseek error: {0}")]
    /// The DeepSeek API returned an error.
    DeepseekError(#[from] DeepseekError),

    #[cfg(feature = "ollama")]
    #[error("Ollama error: {0}")]
    /// The Ollama client returned an error.
    OllamaError(#[from] OllamaError),

    #[error("Network request failed: {0}")]
    /// An HTTP request failed.
    RequestError(#[from] ReqwestError),

    #[error("JSON serialization/deserialization error: {0}")]
    /// JSON serialization or deserialization failed.
    SerdeError(#[from] SerdeJsonError),

    #[error("IO error: {0}")]
    /// A local I/O operation failed.
    IoError(#[from] std::io::Error),

    #[error("Operation timed out")]
    /// An operation exceeded its allotted time.
    Timeout(#[from] Elapsed),

    #[error("Invalid URL: {0}")]
    /// A configured or returned URL was invalid.
    InvalidUrl(String),

    #[error("Content not found in response: Expected at {0}")]
    /// An expected field was absent from a provider response.
    ContentNotFound(String),

    #[error("Parsing error: {0}")]
    /// Provider data could not be parsed.
    ParsingError(String),

    #[error("Error: {0}")]
    /// An error not represented by a more specific variant.
    OtherError(String),
}
