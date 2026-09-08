//! Common vector store and retriever abstractions.
use async_trait::async_trait;

use langchainx_embedding::schemas::{self, Document};

use crate::{VecStoreOptions, VectorStoreError};

// VectorStore is the trait for saving and querying documents in the
// form of vector embeddings.
#[async_trait]
/// Stores embedded documents and retrieves documents by vector similarity.
pub trait VectorStore: Send + Sync {
    /// Backend-specific operation options.
    type Options;

    /// Embeds and stores documents, returning their backend identifiers.
    async fn add_documents(
        &self,
        docs: &[Document],
        opt: &Self::Options,
    ) -> Result<Vec<String>, VectorStoreError>;

    /// Returns up to `limit` documents most similar to `query`.
    async fn similarity_search(
        &self,
        query: &str,
        limit: usize,
        opt: &Self::Options,
    ) -> Result<Vec<Document>, VectorStoreError>;
}

impl<VS, F> From<VS> for Box<dyn VectorStore<Options = F>>
where
    VS: 'static + VectorStore<Options = F>,
{
    fn from(vector_store: VS) -> Self {
        Box::new(vector_store)
    }
}

#[macro_export]
/// Adds documents with default options or an explicitly supplied options value.
macro_rules! add_documents {
    ($obj:expr, $docs:expr) => {
        $obj.add_documents($docs, &$crate::VecStoreOptions::default())
    };
    ($obj:expr, $docs:expr, $opt:expr) => {
        $obj.add_documents($docs, $opt)
    };
}

#[macro_export]
/// Runs a similarity search with default options or an explicitly supplied options value.
macro_rules! similarity_search {
    ($obj:expr, $query:expr, $limit:expr) => {
        $obj.similarity_search($query, $limit, &$crate::VecStoreOptions::default())
    };
    ($obj:expr, $query:expr, $limit:expr, $opt:expr) => {
        $obj.similarity_search($query, $limit, $opt)
    };
}

// Retriever is a retriever for vector stores.
/// Adapts a [`VectorStore`] to the core retriever interface.
pub struct Retriever<F> {
    vstore: Box<dyn VectorStore<Options = VecStoreOptions<F>>>,
    num_docs: usize,
    options: VecStoreOptions<F>,
}

impl<F> Retriever<F> {
    /// Creates a retriever that returns at most `num_docs` documents per query.
    pub fn new<V: Into<Box<dyn VectorStore<Options = VecStoreOptions<F>>>>>(
        vstore: V,
        num_docs: usize,
    ) -> Self {
        Retriever {
            vstore: vstore.into(),
            num_docs,
            options: VecStoreOptions::<F>::new(),
        }
    }

    /// Sets the options passed to each similarity search.
    pub fn with_options(mut self, options: VecStoreOptions<F>) -> Self {
        self.options = options;
        self
    }
}

#[async_trait]
impl<O: Sync + Send> schemas::Retriever for Retriever<O> {
    async fn get_relevant_documents(
        &self,
        query: &str,
    ) -> Result<Vec<Document>, Box<dyn std::error::Error>> {
        self.vstore
            .similarity_search(query, self.num_docs, &self.options)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
