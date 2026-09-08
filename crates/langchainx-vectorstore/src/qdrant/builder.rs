//! Builder for configuring and optionally creating a Qdrant collection.
use langchainx_embedding::embedding::Embedder;
use qdrant_client::Qdrant;
use qdrant_client::qdrant::{CreateCollectionBuilder, Distance, Filter, VectorParamsBuilder};
use std::error::Error;
use std::sync::Arc;

use super::Store;

/// Configures a Qdrant-backed vector store.
pub struct StoreBuilder {
    client: Option<Qdrant>,
    embedder: Option<Arc<dyn Embedder>>,
    collection_name: Option<String>,
    content_field: String,
    metadata_field: String,
    recreate_collection: bool,
    search_filter: Option<Filter>,
}

impl Default for StoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreBuilder {
    /// Creates a builder with standard payload field names.
    pub fn new() -> Self {
        StoreBuilder {
            client: None,
            embedder: None,
            collection_name: None,
            search_filter: None,
            content_field: "page_content".to_string(),
            metadata_field: "metadata".to_string(),
            recreate_collection: false,
        }
    }

    /// Sets the required Qdrant client.
    pub fn client(mut self, client: Qdrant) -> Self {
        self.client = Some(client);
        self
    }

    /// Sets the required document and query embedder.
    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    /// Sets the required Qdrant collection name.
    /// It is recommended to create a collection in advance, with the required configurations.
    /// <https://qdrant.tech/documentation/concepts/collections/#create-a-collection>
    ///
    /// If the collection doesn't exist, it will be created with the embedding provider's dimension
    /// and Cosine similarity metric.
    pub fn collection_name(mut self, collection_name: &str) -> Self {
        self.collection_name = Some(collection_name.to_string());
        self
    }

    /// Sets the payload field used for document metadata.
    pub fn metadata_field(mut self, metadata_field: &str) -> Self {
        self.metadata_field = metadata_field.to_string();
        self
    }

    /// Sets the payload field used for document content.
    pub fn content_field(mut self, content_field: &str) -> Self {
        self.content_field = content_field.to_string();
        self
    }

    /// Controls whether an existing collection is deleted and recreated.
    pub fn recreate_collection(mut self, recreate_collection: bool) -> Self {
        self.recreate_collection = recreate_collection;
        self
    }

    /// Sets the Qdrant filter applied to similarity searches.
    /// <https://qdrant.tech/documentation/concepts/filtering/>
    /// Instance of use `qdrant_client::qdrant::Filter`
    pub fn search_filter(mut self, search_filter: Filter) -> Self {
        self.search_filter = Some(search_filter);
        self
    }

    /// Builds the store, creating or recreating its cosine-distance collection when needed.
    pub async fn build(mut self) -> Result<Store, Box<dyn Error>> {
        let client = self.client.take().ok_or("'client' is required")?;
        let embedder = self.embedder.take().ok_or("'embedder' is required")?;
        let collection_name = self
            .collection_name
            .take()
            .ok_or("'collection_name' is required")?;

        let collection_exists = client.collection_exists(&collection_name).await?;

        // Delete the collection if it exists and recreate_collection flag is set
        if collection_exists && self.recreate_collection {
            client.delete_collection(&collection_name).await?;
        }

        // Create the collection if it doesn't exist or recreate_collection flag is set
        if !collection_exists || self.recreate_collection {
            // Embed some text to get the dimension of the embeddings
            let embeddings = embedder
                .embed_query("Text to retrieve embeddings dimension")
                .await?;
            let embeddings_dimension = embeddings.len() as u64;

            client
                .create_collection(
                    CreateCollectionBuilder::new(&collection_name).vectors_config(
                        VectorParamsBuilder::new(embeddings_dimension, Distance::Cosine),
                    ),
                )
                .await?;
        }

        Ok(Store {
            client,
            embedder,
            collection_name,
            search_filter: self.search_filter,
            content_field: self.content_field,
            metadata_field: self.metadata_field,
        })
    }
}
