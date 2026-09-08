//! Builder for OpenSearch k-NN vector stores.
use langchainx_embedding::embedding::Embedder;
use opensearch::OpenSearch;
use std::error::Error;
use std::sync::Arc;

use super::Store;

/// Configures an OpenSearch-backed vector store.
pub struct StoreBuilder {
    client: Option<OpenSearch>,
    embedder: Option<Arc<dyn Embedder>>,
    k: i32,
    index: Option<String>,
    vector_field: String,
    content_field: String,
}

impl Default for StoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreBuilder {
    // Returns a new StoreBuilder instance with default values for each option
    /// Creates a builder with default k-NN and field settings.
    pub fn new() -> Self {
        StoreBuilder {
            client: None,
            embedder: None,
            k: 2,
            index: None,
            vector_field: "vector_field".to_string(),
            content_field: "page_content".to_string(),
        }
    }

    /// Sets the required OpenSearch client.
    pub fn client(mut self, client: OpenSearch) -> Self {
        self.client = Some(client);
        self
    }

    /// Sets the required document and query embedder.
    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    /// Sets the neighbor count sent to the OpenSearch k-NN query.
    pub fn k(mut self, k: i32) -> Self {
        self.k = k;
        self
    }

    /// Sets the required OpenSearch index name.
    pub fn index(mut self, index: &str) -> Self {
        self.index = Some(index.to_string());
        self
    }

    /// Sets the index field containing embedding vectors.
    pub fn vector_field(mut self, vector_field: &str) -> Self {
        self.vector_field = vector_field.to_string();
        self
    }

    /// Sets the index field containing document text.
    pub fn content_field(mut self, content_field: &str) -> Self {
        self.content_field = content_field.to_string();
        self
    }

    /// Builds the store, failing if the client, embedder, or index is missing.
    pub async fn build(self) -> Result<Store, Box<dyn Error>> {
        if self.client.is_none() {
            return Err("Client is required".into());
        }

        if self.embedder.is_none() {
            return Err("Embedder is required".into());
        }

        if self.index.is_none() {
            return Err("Index is required".into());
        }

        Ok(Store {
            client: self.client.unwrap(),
            embedder: self.embedder.unwrap(),
            k: self.k,
            index: self.index.unwrap(),
            vector_field: self.vector_field,
            content_field: self.content_field,
        })
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use langchainx_embedding::embedding::{Embedder, EmbedderError};

    use super::*;

    struct DummyEmbedder;

    #[async_trait]
    impl Embedder for DummyEmbedder {
        async fn embed_documents(&self, _docs: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(vec![])
        }

        async fn embed_query(&self, _query: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(vec![])
        }
    }

    fn err_msg<T>(r: Result<T, Box<dyn std::error::Error>>) -> String {
        match r {
            Ok(_) => panic!("expected Err, got Ok"),
            Err(e) => e.to_string(),
        }
    }

    #[tokio::test]
    async fn build_without_client_returns_error() {
        let result = StoreBuilder::new().index("my-index").build().await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Client"));
    }

    #[tokio::test]
    async fn build_without_embedder_returns_error() {
        let client = opensearch::OpenSearch::default();
        let result = StoreBuilder::new()
            .client(client)
            // no .embedder()
            .index("my-index")
            .build()
            .await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Embedder"));
    }

    #[tokio::test]
    async fn build_without_index_returns_error() {
        let client = opensearch::OpenSearch::default();
        let result = StoreBuilder::new()
            .client(client)
            .embedder(DummyEmbedder)
            // no .index()
            .build()
            .await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Index"));
    }
}
