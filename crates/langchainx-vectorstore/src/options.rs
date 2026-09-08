//! Per-operation options shared by vector store backends.
use std::sync::Arc;

use langchainx_embedding::embedding::embedder_trait::Embedder;
use serde_json::Value;

/// Optional namespace, score threshold, filters, and embedder overrides for an operation.
///
/// # Usage
/// ```rust,ignore
/// let options = VecStoreOptions::new()
///     .with_name_space("my_custom_namespace")
///     .with_score_threshold(0.5)
///     .with_filters(json!({"genre": "Sci-Fi"}))
///     .with_embedder(my_embedder);
/// ```
pub struct VecStoreOptions<F> {
    /// Backend namespace or collection override.
    pub name_space: Option<String>,
    /// Minimum similarity score accepted by a search.
    pub score_threshold: Option<f32>,
    /// Backend-specific search filters.
    pub filters: Option<F>,
    /// Embedder to use instead of the store's configured embedder.
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl Default for VecStoreOptions<Value> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F> VecStoreOptions<F> {
    /// Creates options with no overrides.
    pub fn new() -> Self {
        VecStoreOptions {
            name_space: None,
            score_threshold: None,
            filters: None,
            embedder: None,
        }
    }

    /// Sets the namespace or collection used by the operation.
    pub fn with_name_space<S: Into<String>>(mut self, name_space: S) -> Self {
        self.name_space = Some(name_space.into());
        self
    }

    /// Sets the minimum accepted similarity score.
    pub fn with_score_threshold(mut self, score_threshold: f32) -> Self {
        self.score_threshold = Some(score_threshold);
        self
    }

    /// Sets backend-specific search filters.
    pub fn with_filters(mut self, filters: F) -> Self {
        self.filters = Some(filters);
        self
    }

    /// Overrides the store's embedder for this operation.
    pub fn with_embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }
}
