use std::{error::Error, sync::Arc};

use surrealdb::{Connection, Surreal};

use langchainx_embedding::embedding::embedder_trait::Embedder;

use super::Store;

pub struct StoreBuilder<C: Connection> {
    db: Option<Surreal<C>>,
    collection_name: String,
    collection_table_name: Option<String>,
    collection_metadata_key_name: Option<String>,
    vector_dimensions: i32,
    embedder: Option<Arc<dyn Embedder>>,
    schemafull: bool,
}

impl<C: Connection> Default for StoreBuilder<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Connection> StoreBuilder<C> {
    /// Create a new StoreBuilder optimized for SurrealDB. Refer to `new_with_compatiblity()` if
    /// you are looking to connect to store created by python version of langchain.
    /// * table is singular - "document" instead of "documents"
    /// * uses single table instead of multiple tables
    /// * creates a schemafull table required for faster indexing.
    ///   https://github.com/surrealdb/surrealdb/issues/2013
    pub fn new() -> Self {
        StoreBuilder {
            db: None,
            collection_name: "document".to_string(),
            collection_table_name: Some("document".to_string()),
            collection_metadata_key_name: Some("collection".to_string()),
            vector_dimensions: 0,
            embedder: None,
            schemafull: true,
        }
    }

    /// Create a new StoreBuilder with compatibility with python version of langchain
    pub fn new_with_compatiblity() -> Self {
        StoreBuilder {
            db: None,
            collection_name: "documents".to_string(),
            collection_table_name: None,
            collection_metadata_key_name: None,
            vector_dimensions: 0,
            embedder: None,
            schemafull: false,
        }
    }

    /// Use surrealdb
    /// ```no_run
    /// use langchainx_vectorstore::surrealdb::StoreBuilder;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let surrealdb_config = surrealdb::opt::Config::new()
    ///         .set_strict(true)
    ///         .capabilities(surrealdb::opt::capabilities::Capabilities::all())
    ///         .user(surrealdb::opt::auth::Root {
    ///             username: "username".into(),
    ///             password: "password".into()
    ///         });
    ///     let db = surrealdb::engine::any::connect(
    ///         ("ws://127.0.0.1:8000", surrealdb_config)
    ///     ).await.unwrap();
    ///     let store = StoreBuilder::new().db(db).vector_dimensions(1000).build().await.unwrap();
    ///     store.initialize().await.unwrap();
    /// }
    /// ```
    pub fn db(mut self, db: Surreal<C>) -> Self {
        self.db = Some(db);
        self
    }

    pub fn collection_name(mut self, collection_name: &str) -> Self {
        self.collection_name = collection_name.into();
        self
    }

    /// Setting collection_table_name to None, creates table per collection. Set to some value if
    /// you would like to reuse table. Reusing table is not compatible with python version of
    /// langchain.
    pub fn collection_table_name(mut self, collection_table_name: Option<String>) -> Self {
        self.collection_table_name = collection_table_name;
        self
    }

    pub fn collection_metadata_key_name(
        mut self,
        collection_metadata_key_name: Option<String>,
    ) -> Self {
        self.collection_metadata_key_name = collection_metadata_key_name;
        self
    }

    pub fn vector_dimensions(mut self, vector_dimensions: i32) -> Self {
        self.vector_dimensions = vector_dimensions;
        self
    }

    pub fn schemafull(mut self, schemafull: bool) -> Self {
        self.schemafull = schemafull;
        self
    }

    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    /// Build the Store. Returns `Err` if embedder or db is missing.
    pub async fn build(self) -> Result<Store<C>, Box<dyn Error>> {
        if self.embedder.is_none() {
            return Err("Embedder is required".into());
        }

        if self.db.is_none() {
            return Err("Db is required".into());
        }

        Ok(Store {
            db: self.db.unwrap(),
            collection_name: self.collection_name,
            collection_table_name: self.collection_table_name,
            collection_metadata_key_name: self.collection_metadata_key_name,
            vector_dimensions: self.vector_dimensions,
            embedder: self.embedder.unwrap(),
            schemafull: self.schemafull,
        })
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use langchainx_embedding::embedding::{Embedder, EmbedderError};
    use surrealdb::engine::any::Any;

    use super::*;

    struct DummyEmbedder;

    #[async_trait]
    impl Embedder for DummyEmbedder {
        async fn embed_documents(
            &self,
            _docs: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(vec![])
        }

        async fn embed_query(&self, _query: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(vec![])
        }
    }

    fn err_msg<T, E: std::fmt::Display>(r: Result<T, E>) -> String {
        match r {
            Ok(_) => panic!("expected Err, got Ok"),
            Err(e) => e.to_string(),
        }
    }

    #[tokio::test]
    async fn build_without_embedder_returns_error() {
        let result = StoreBuilder::<Any>::new()
            // no .embedder(), no .db()
            .build()
            .await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Embedder"));
    }

    #[tokio::test]
    async fn build_without_db_returns_error() {
        let result = StoreBuilder::<Any>::new()
            .embedder(DummyEmbedder)
            // no .db()
            .build()
            .await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Db"));
    }

    #[test]
    fn new_with_compatibility_sets_expected_defaults() {
        let builder = StoreBuilder::<Any>::new_with_compatiblity();
        assert_eq!(builder.collection_name, "documents");
        assert!(!builder.schemafull);
    }
}
