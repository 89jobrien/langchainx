use std::sync::Arc;

use async_trait::async_trait;
use langchainx_core::schemas::Document;
use langchainx_vectorstore::{VecStoreOptions, VectorStore, VectorStoreError};
use serde_json::Value;
use tokio::sync::Mutex;

/// A deterministic vector store that returns inserted documents in insertion order.
#[derive(Clone, Debug, Default)]
pub struct InMemoryVectorStore {
    documents: Arc<Mutex<Vec<Document>>>,
}

impl InMemoryVectorStore {
    /// Creates an empty in-memory store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a snapshot of all inserted documents.
    pub async fn documents(&self) -> Vec<Document> {
        self.documents.lock().await.clone()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    type Options = VecStoreOptions<Value>;

    async fn add_documents(
        &self,
        docs: &[Document],
        _options: &Self::Options,
    ) -> Result<Vec<String>, VectorStoreError> {
        let mut stored = self.documents.lock().await;
        let start = stored.len();
        stored.extend_from_slice(docs);
        Ok((start..stored.len())
            .map(|index| index.to_string())
            .collect())
    }

    async fn similarity_search(
        &self,
        _query: &str,
        limit: usize,
        _options: &Self::Options,
    ) -> Result<Vec<Document>, VectorStoreError> {
        Ok(self
            .documents
            .lock()
            .await
            .iter()
            .take(limit)
            .cloned()
            .collect())
    }
}
