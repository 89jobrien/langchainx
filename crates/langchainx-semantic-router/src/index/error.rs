//! Errors produced by semantic route indexes.
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors produced while storing or retrieving routes.
pub enum IndexError {
    #[error("No Emedding on Route: {0}")]
    /// A route did not contain embedding vectors.
    MissingEmbedding(String),

    #[error("Error: {0}")]
    /// An index-specific error not represented by another variant.
    OtherError(String),

    #[error("No Route found: {0}")]
    /// No route exists with the requested name.
    RouterNotFound(String),
}
