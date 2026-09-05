//! Errors produced while building and executing chains.
use thiserror::Error;

use crate::{language_models::LLMError, output_parsers::OutputParserError, prompt::PromptError};

#[derive(Error, Debug)]
/// An error produced by chain configuration, input validation, or execution.
pub enum ChainError {
    #[error("LLM error: {0}")]
    /// The configured language model failed.
    LLMError(#[from] LLMError),

    #[error("Retriever error: {0}")]
    /// A retriever failed to load relevant documents.
    RetrieverError(String),

    #[error("OutputParser error: {0}")]
    /// The configured output parser rejected a generation.
    OutputParser(#[from] OutputParserError),

    #[error("Prompt error: {0}")]
    /// Prompt formatting or rendering failed.
    PromptError(#[from] PromptError),

    #[error("Missing Object On Builder: {0}")]
    /// A required builder component was not configured.
    MissingObject(String),

    #[error("Missing input variable `{key}`: expected {expected:?}, got {provided:?}")]
    /// One of the chain's required input variables was absent.
    MissingInputVariable {
        /// The first required key that was missing.
        key: String,
        /// All keys required by the chain.
        expected: Vec<String>,
        /// Keys present in the supplied input.
        provided: Vec<String>,
    },

    #[error("Serde json error: {0}")]
    /// Chain data could not be serialized or deserialized as JSON.
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Incorrect input variable: expected type {expected_type}, {source}")]
    /// An input value could not be decoded as the type expected by the chain.
    IncorrectInputVariable {
        /// The JSON decoding failure.
        source: serde_json::Error,
        /// A human-readable name for the required input type.
        expected_type: String,
    },

    #[error("Error: {0}")]
    /// A chain-specific failure not represented by another variant.
    OtherError(String),

    #[error("Database error: {0}")]
    /// A database operation failed.
    DatabaseError(String),

    #[error("Agent error: {0}")]
    /// An agent used as a chain failed while planning or running a tool.
    AgentError(String),
}
