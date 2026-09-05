//! Abstraction for retrieving documents relevant to a query.
use std::error::Error;

use async_trait::async_trait;

use super::Document;

#[async_trait]
/// Finds documents relevant to natural-language queries.
pub trait Retriever: Sync + Send {
    /// Retrieves documents relevant to `query`.
    async fn get_relevant_documents(&self, query: &str) -> Result<Vec<Document>, Box<dyn Error>>;
}

impl<R> From<R> for Box<dyn Retriever>
where
    R: Retriever + 'static,
{
    fn from(retriever: R) -> Self {
        Box::new(retriever)
    }
}
