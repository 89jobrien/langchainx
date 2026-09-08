//! The provider-independent interface for producing vector embeddings.
use async_trait::async_trait;

use super::EmbedderError;

#[async_trait]
/// Converts document or query text into numeric embedding vectors.
pub trait Embedder: Send + Sync {
    /// Embeds each document and returns vectors in the same order as the input.
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError>;
    /// Embeds a single query string.
    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError>;
}
