//! Errors returned by embedding providers.
use async_openai::error::OpenAIError;
#[cfg(feature = "mistralai")]
use mistralai_client::v1::error::{ApiError, ClientError};
#[cfg(feature = "ollama")]
use ollama_rs::error::OllamaError;
use reqwest::{Error as ReqwestError, StatusCode};
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors that can occur while configuring or requesting embeddings.
pub enum EmbedderError {
    #[error("Network request failed: {0}")]
    /// An HTTP request failed before a provider response was received.
    RequestError(#[from] ReqwestError),

    #[error("OpenAI error: {0}")]
    /// The OpenAI client returned an error.
    OpenAIError(#[from] OpenAIError),

    #[error("URL parsing error: {0}")]
    /// A provider URL could not be parsed.
    UrlParseError(#[from] url::ParseError),

    #[error("HTTP error: {status_code} {error_message}")]
    /// A provider returned an unsuccessful HTTP response.
    HttpError {
        /// The HTTP response status.
        status_code: StatusCode,
        /// The provider's error message.
        error_message: String,
    },

    #[error("FastEmbed error: {0}")]
    /// The local FastEmbed model failed to initialize or embed input.
    FastEmbedError(String),

    #[cfg(feature = "ollama")]
    #[error("Ollama error: {0}")]
    /// The Ollama client returned an error.
    OllamaError(#[from] OllamaError),

    #[cfg(feature = "mistralai")]
    #[error("MistralAI Client error: {0}")]
    /// The Mistral AI client could not be configured.
    MistralAIClientError(#[from] ClientError),

    #[cfg(feature = "mistralai")]
    #[error("MistralAI API error: {0}")]
    /// The Mistral AI API returned an error.
    MistralAIApiError(#[from] ApiError),
}
