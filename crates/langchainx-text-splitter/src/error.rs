//! Errors produced while configuring or running text splitters.
use text_splitter::ChunkConfigError;
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors produced by text splitters.
pub enum TextSplitterError {
    #[error("Empty input text")]
    /// Input text was empty where content was required.
    EmptyInputText,

    #[error("Mismatch metadata and text")]
    /// The number of metadata maps did not match the number of input texts.
    MetadataTextMismatch,

    #[error("Tokenizer not found")]
    /// The configured tokenizer name is unsupported.
    TokenizerNotFound,

    #[error("Tokenizer creation failed due to invalid tokenizer")]
    /// The configured tokenizer could not be initialized.
    InvalidTokenizer,

    #[error("Tokenizer creation failed due to invalid model")]
    /// The configured model could not be resolved to a tokenizer.
    InvalidModel,

    #[error("Invalid chunk overlap and size")]
    /// Chunk size and overlap do not form a valid configuration.
    InvalidSplitterOptions,

    #[error("Error: {0}")]
    /// A splitter-specific error not represented by another variant.
    OtherError(String),
}

impl From<ChunkConfigError> for TextSplitterError {
    fn from(_: ChunkConfigError) -> Self {
        Self::InvalidSplitterOptions
    }
}
