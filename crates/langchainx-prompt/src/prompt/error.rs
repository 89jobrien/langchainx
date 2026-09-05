//! Errors produced while formatting prompt templates and messages.
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors produced while constructing a prompt.
pub enum PromptError {
    #[error("Variable {0} is missing from input variables")]
    /// A required template variable was not supplied.
    MissingVariable(String),

    #[error("Serialization error: {0}")]
    /// Message placeholder data could not be deserialized.
    SerializationError(#[from] SerdeJsonError),

    #[error("Error: {0}")]
    /// A prompt error not covered by a structured variant.
    OtherError(String),
}
