//! Errors produced while configuring and executing semantic routes.
use thiserror::Error;

use langchainx_chain::ChainError;
use langchainx_chain::language_models::LLMError;
use langchainx_embedding::EmbedderError;
use serde_json::Error as SerdeJsonError;

use crate::IndexError;

#[derive(Error, Debug)]
/// Errors produced while constructing a route definition.
pub enum RouterBuilderError {
    #[error(
        "Invalid Router configuration: at least one of utterances or embedding must be provided, \
         and utterances cannot be an empty vector."
    )]
    /// Neither usable utterances nor precomputed embeddings were supplied.
    InvalidConfiguration,
}

#[derive(Error, Debug)]
/// Errors produced while building a route layer.
pub enum RouteLayerBuilderError {
    #[error("Route layer should have an embedder")]
    /// No embedder was configured.
    MissingEmbedder,

    #[error("Route layer should have an LLM")]
    /// No language model was configured.
    MissingLLM,

    #[error("Missing Index")]
    /// No route index was configured.
    MissingIndex,

    #[error("Route layer error: {0}")]
    /// Route-layer initialization failed.
    RouteLayerError(#[from] RouteLayerError),

    #[error("Index error: {0}")]
    /// Initial route indexing failed.
    IndexError(#[from] IndexError),

    #[error("Embedding error: {0}")]
    /// Initial route embedding failed.
    EmbeddingError(#[from] EmbedderError),

    #[error("Chain error: {0}")]
    /// The tool-input chain could not be built or invoked.
    ChainError(#[from] ChainError),
}

#[derive(Error, Debug)]
/// Errors produced while updating or querying a route layer.
pub enum RouteLayerError {
    #[error("Embedding error: {0}")]
    /// Query or route embedding failed.
    EmbeddingError(#[from] EmbedderError),

    #[error("Index error: {0}")]
    /// An index operation failed.
    IndexError(#[from] IndexError),

    #[error("LLM error: {0}")]
    /// A direct language-model operation failed.
    LLMError(#[from] LLMError),

    #[error("Serialization error: {0}")]
    /// Tool input serialization or deserialization failed.
    SerializationError(#[from] SerdeJsonError),

    #[error("Chain error: {0}")]
    /// Tool-input chain execution failed.
    ChainError(#[from] ChainError),
}
