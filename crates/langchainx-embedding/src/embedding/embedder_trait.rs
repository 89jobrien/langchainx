use async_trait::async_trait;

use super::EmbedderError;

// TODO(#36): add conformance test suite for Embedder trait -- verify
//   embed_query output dimension matches embed_documents per-doc dimension,
//   empty input returns empty vec, consistent results for identical input.
#[async_trait]
pub trait Embedder: Send + Sync {
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError>;
    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError>;
}
