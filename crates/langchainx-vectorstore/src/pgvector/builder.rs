//! Builder and schema initialization for PostgreSQL pgvector stores.
use std::{collections::HashMap, env, error::Error, sync::Arc};

use serde_json::{Value, json};
use sqlx::{Pool, Postgres, Row, Transaction, postgres::PgPoolOptions};

use langchainx_embedding::embedding::embedder_trait::Embedder;

use crate::VecStoreOptions;

use super::{
    HNSWIndex, PG_LOCK_ID_COLLECTION_TABLE, PG_LOCK_ID_EMBEDDING_TABLE, PG_LOCKID_EXTENSION,
    PgFilter, PgOptions, Store,
};

const DEFAULT_COLLECTION_NAME: &str = "langchain";
const DEFAULT_PRE_DELETE_COLLECTION: bool = false;
const DEFAULT_EMBEDDING_STORE_TABLE_NAME: &str = "langchain_pg_embedding";
const DEFAULT_COLLECTION_STORE_TABLE_NAME: &str = "langchain_pg_collection";

/// Configures a pgvector store and initializes its extension, tables, and indexes.
pub struct StoreBuilder<F> {
    pool: Option<Pool<Postgres>>,
    embedder: Option<Arc<dyn Embedder>>,
    connection_url: Option<String>,
    vector_dimensions: i32,
    pre_delete_collection: bool,
    embedder_table_name: String,
    collection_name: String,
    collection_table_name: String,
    collection_metadata: HashMap<String, Value>,
    vstore_options: VecStoreOptions<F>,
    hns_index: Option<HNSWIndex>,
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use async_trait::async_trait;
    use langchainx_embedding::embedding::{Embedder, EmbedderError};

    use super::*;

    struct DummyEmbedder;

    #[async_trait]
    impl Embedder for DummyEmbedder {
        async fn embed_documents(
            &self,
            _documents: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(Vec::new())
        }

        async fn embed_query(&self, _text: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(Vec::new())
        }
    }

    #[tokio::test]
    async fn build_rejects_unsafe_table_names_before_connecting() {
        let result = StoreBuilder::new()
            .embedder(DummyEmbedder)
            .embedder_table_name("documents; DROP TABLE users")
            .build()
            .await;

        let error = result.err().expect("unsafe table name should fail");
        assert!(error.to_string().contains("Invalid PostgreSQL identifier"));
    }

    #[tokio::test]
    async fn build_rejects_unsafe_hnsw_operator_class_before_connecting() {
        let result = StoreBuilder::new()
            .embedder(DummyEmbedder)
            .hns_index(HNSWIndex::new(
                16,
                64,
                "vector_cosine_ops); DROP TABLE users",
            ))
            .build()
            .await;

        let error = result.err().expect("unsafe operator class should fail");
        assert!(
            error
                .to_string()
                .contains("Unsupported HNSW distance function")
        );
    }
}

impl Default for StoreBuilder<PgFilter> {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreBuilder<PgFilter> {
    // Returns a new StoreBuilder instance with default values for each option
    /// Creates a builder with LangChain-compatible table and collection names.
    pub fn new() -> Self {
        StoreBuilder {
            pool: None,
            embedder: None,
            connection_url: None,
            vector_dimensions: 0,
            pre_delete_collection: DEFAULT_PRE_DELETE_COLLECTION,
            embedder_table_name: DEFAULT_EMBEDDING_STORE_TABLE_NAME.into(),
            collection_name: DEFAULT_COLLECTION_NAME.into(),
            collection_table_name: DEFAULT_COLLECTION_STORE_TABLE_NAME.into(),
            collection_metadata: HashMap::new(),
            vstore_options: VecStoreOptions::new(),
            hns_index: None,
        }
    }

    /// Uses an existing PostgreSQL connection pool.
    pub fn pool(mut self, pool: Pool<Postgres>) -> Self {
        self.pool = Some(pool);
        self
    }

    /// Sets the required document and query embedder.
    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    /// Sets the PostgreSQL URL used when no pool is supplied.
    pub fn connection_url(mut self, connection_url: &str) -> Self {
        self.connection_url = Some(connection_url.into());
        self
    }

    /// Sets the optional fixed dimensions of the pgvector column.
    pub fn vector_dimensions(mut self, vector_dimensions: i32) -> Self {
        self.vector_dimensions = vector_dimensions;
        self
    }

    /// Controls whether the named collection is deleted before initialization.
    pub fn pre_delete_collection(mut self, pre_delete_collection: bool) -> Self {
        self.pre_delete_collection = pre_delete_collection;
        self
    }

    /// Sets the table containing documents and embeddings.
    pub fn embedder_table_name(mut self, embedder_table_name: &str) -> Self {
        self.embedder_table_name = embedder_table_name.into();
        self
    }

    /// Sets the logical collection name.
    pub fn collection_name(mut self, collection_name: &str) -> Self {
        self.collection_name = collection_name.into();
        self
    }

    /// Sets the table containing collection metadata.
    pub fn collection_table_name(mut self, collection_table_name: &str) -> Self {
        self.collection_table_name = collection_table_name.into();
        self
    }

    /// Stores operation options on the builder; the current build output does not consume them.
    pub fn vstore_options(mut self, vstore_options: PgOptions) -> Self {
        self.vstore_options = vstore_options;
        self
    }

    /// Sets metadata stored with the collection record.
    pub fn collection_metadata(mut self, collection_metadata: HashMap<String, Value>) -> Self {
        self.collection_metadata = collection_metadata;
        self
    }

    /// Enables an HNSW embedding index with the supplied settings.
    pub fn hns_index(mut self, hns_index: HNSWIndex) -> Self {
        self.hns_index = Some(hns_index);
        self
    }

    // Finalize the builder and construct the Store object
    /// Initializes the pgvector schema and builds the store.
    pub async fn build(self) -> Result<Store, Box<dyn Error>> {
        if self.embedder.is_none() {
            return Err("Embedder is required".into());
        }
        validate_postgres_identifier(&self.embedder_table_name)?;
        validate_postgres_identifier(&self.collection_table_name)?;
        if let Some(index) = &self.hns_index {
            validate_distance_function(&index.distance_function)?;
        }
        let pool = self.get_pool().await?;
        let mut tx = pool.begin().await?;
        self.create_vector_extension_if_not_exists(&mut tx).await?;
        self.create_collection_table_if_not_exists(&mut tx).await?;
        self.create_embedding_table_if_not_exists(&mut tx).await?;

        if self.pre_delete_collection {
            self.remove_collection(&mut tx).await?;
        }

        let collection_uuid = self.create_or_get_collection(&mut tx).await?;

        tx.commit().await?;

        Ok(Store {
            pool,
            embedder: self.embedder.unwrap(),
            collection_name: self.collection_name,
            collection_uuid,
            collection_table_name: self.collection_table_name,
            embedder_table_name: self.embedder_table_name,
        })
    }

    async fn get_pool(&self) -> Result<Pool<Postgres>, Box<dyn Error>> {
        match &self.pool {
            Some(existing_pool) => {
                // If `self.pool` is Some, use the existing pool
                Ok(existing_pool.clone())
            }
            None => {
                let connection_url = match self.connection_url {
        Some(ref url) if !url.is_empty() => url.clone(),
        _ => env::var("PGVECTOR_CONNECTION_STRING")
                            .map_err(|_| "PGVECTOR_CONNECTION_STRING environment variable not set, and no connection URL provided.")?};

                // Check if the resolved `connection_url` is empty
                if connection_url.is_empty() {
                    return Err("Connection URL is empty.".into());
                }

                // Create a new pool
                let new_pool = PgPoolOptions::new()
                    .connect(&connection_url)
                    .await
                    .map_err(|e| format!("Failed to create a new connection pool: {}", e))?;
                Ok(new_pool)
            }
        }
    }

    async fn create_or_get_collection(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<String, Box<dyn Error>> {
        let sql = format!(
            r#"INSERT INTO {} (uuid, name, cmetadata)
        VALUES($1, $2, $3) ON CONFLICT (name) DO
        UPDATE SET cmetadata = $3"#,
            self.collection_table_name
        );
        sqlx::query(&sql)
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&self.collection_name)
            .bind(json!(&self.collection_metadata))
            .execute(&mut **tx)
            .await?;

        let sql = format!(
            r#"SELECT uuid FROM {} WHERE name = $1 ORDER BY name limit 1"#,
            self.collection_table_name
        );
        let row = sqlx::query(&sql)
            .bind(&self.collection_name)
            .fetch_one(&mut **tx)
            .await?;

        Ok(row.get(0))
    }

    async fn remove_collection(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query(&format!(
            "DELETE FROM {} WHERE name = $1",
            self.collection_table_name
        ))
        .bind(&self.collection_name)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Creates the pgvector extension while holding the shared advisory lock.
    pub async fn create_vector_extension_if_not_exists(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        // Acquire an advisory lock to prevent concurrent creation of the vector extension
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(PG_LOCKID_EXTENSION)
            .execute(&mut **tx)
            .await?;

        // Create the vector extension if it doesn't already exist
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    async fn create_collection_table_if_not_exists(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        // inspired by
        // https://github.com/langchain-ai/langchain/blob/v0.0.340/libs/langchain/langchain/vectorstores/pgvector.py#L167
        // The advisor lock fixes issue arising from concurrent
        // creation of the vector extension.
        // https://github.com/langchain-ai/langchain/issues/12933
        // For more information see:
        // https://www.postgresql.org/docs/16/explicit-locking.html#ADVISORY-LOCKS
        let create_extension_sql = "CREATE EXTENSION IF NOT EXISTS vector".to_string();
        sqlx::query(&create_extension_sql)
            .execute(&mut **tx)
            .await?;

        // Use advisory lock as before
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(PG_LOCK_ID_COLLECTION_TABLE)
            .execute(&mut **tx)
            .await?;

        // Now, create the table
        let create_table_sql = format!(
            r#"CREATE TABLE IF NOT EXISTS {} (
        name VARCHAR,
        cmetadata JSON,
        "uuid" TEXT NOT NULL,
        UNIQUE (name),
        PRIMARY KEY (uuid)
    )"#,
            self.collection_table_name
        );
        sqlx::query(&create_table_sql).execute(&mut **tx).await?;

        Ok(())
    }

    async fn create_embedding_table_if_not_exists(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), Box<dyn Error>> {
        sqlx::query("SELECT pg_advisory_xact_lock($1)")
            .bind(PG_LOCK_ID_EMBEDDING_TABLE)
            .execute(&mut **tx)
            .await?;

        let mut vector_dimensions = String::from("");
        if self.vector_dimensions > 0 {
            vector_dimensions = format!("({})", self.vector_dimensions);
        }

        let sql = format!(
            r#"CREATE TABLE IF NOT EXISTS {}
             (collection_id TEXT,
             embedding VECTOR{},
             document VARCHAR,
             cmetadata JSON,
             "uuid" TEXT NOT NULL,
             CONSTRAINT langchain_pg_embedding_collection_id_fkey
             FOREIGN KEY (collection_id) REFERENCES {}("uuid") ON DELETE CASCADE,
             PRIMARY KEY ("uuid"))"#,
            self.embedder_table_name, vector_dimensions, self.collection_table_name
        );

        sqlx::query(&sql).execute(&mut **tx).await?;

        let sql = format!(
            r#"CREATE INDEX IF NOT EXISTS {}_collection_id ON {} (collection_id)"#,
            self.embedder_table_name, self.embedder_table_name
        );
        sqlx::query(&sql).execute(&mut **tx).await?;

        // See this for more details on HNWS indexes: https://github.com/pgvector/pgvector#hnsw
        if let Some(hns_index) = &self.hns_index {
            let mut sql = format!(
                r#"CREATE INDEX IF NOT EXISTS {}_embedding_hnsw ON {} USING hnsw (embedding {})"#,
                self.embedder_table_name, self.embedder_table_name, hns_index.distance_function
            );
            if hns_index.m > 0 && hns_index.ef_construction > 0 {
                sql = format!(
                    "{} WITH (m={}, ef_construction = {})",
                    sql, hns_index.m, hns_index.ef_construction
                );
            }
            sqlx::query(&sql).execute(&mut **tx).await?;
        }

        Ok(())
    }
}

fn validate_postgres_identifier(identifier: &str) -> Result<(), Box<dyn Error>> {
    let mut chars = identifier.chars();
    let valid = chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|character| character == '_' || character.is_ascii_alphanumeric());
    if !valid {
        return Err(format!("Invalid PostgreSQL identifier: {identifier}").into());
    }
    Ok(())
}

fn validate_distance_function(distance_function: &str) -> Result<(), Box<dyn Error>> {
    match distance_function {
        "vector_l2_ops" | "vector_ip_ops" | "vector_cosine_ops" | "vector_l1_ops" => Ok(()),
        _ => Err(format!("Unsupported HNSW distance function: {distance_function}").into()),
    }
}
