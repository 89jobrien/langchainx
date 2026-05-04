use langchainx_embedding::embedding::EmbedderError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorStoreError {
    #[error("Embedding error: {0}")]
    EmbedderError(#[from] EmbedderError),

    #[error("Serialization error: {0}")]
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Error: {0}")]
    OtherError(String),
}

#[cfg(any(feature = "postgres", feature = "sqlite-vss", feature = "sqlite-vec"))]
impl From<sqlx::Error> for VectorStoreError {
    fn from(e: sqlx::Error) -> Self {
        VectorStoreError::ConnectionError(e.to_string())
    }
}

#[cfg(feature = "qdrant")]
impl From<qdrant_client::QdrantError> for VectorStoreError {
    fn from(e: qdrant_client::QdrantError) -> Self {
        VectorStoreError::ConnectionError(e.to_string())
    }
}

#[cfg(feature = "opensearch")]
impl From<opensearch::Error> for VectorStoreError {
    fn from(e: opensearch::Error) -> Self {
        VectorStoreError::ConnectionError(e.to_string())
    }
}

#[cfg(feature = "surrealdb")]
impl From<surrealdb::Error> for VectorStoreError {
    fn from(e: surrealdb::Error) -> Self {
        VectorStoreError::ConnectionError(e.to_string())
    }
}
