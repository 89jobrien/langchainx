use std::{error::Error, str::FromStr, sync::Arc};

use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
};

use super::Store;
use langchainx_embedding::embedding::embedder_trait::Embedder;

pub struct StoreBuilder {
    pool: Option<Pool<Sqlite>>,
    connection_url: Option<String>,
    table: String,
    vector_dimensions: i32,
    embedder: Option<Arc<dyn Embedder>>,
}

impl Default for StoreBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl StoreBuilder {
    pub fn new() -> Self {
        StoreBuilder {
            pool: None,
            connection_url: None,
            table: "documents".to_string(),
            vector_dimensions: 0,
            embedder: None,
        }
    }

    pub fn pool(mut self, pool: Pool<Sqlite>) -> Self {
        self.pool = Some(pool);
        self.connection_url = None;
        self
    }

    pub fn connection_url<S: Into<String>>(mut self, connection_url: S) -> Self {
        self.connection_url = Some(connection_url.into());
        self.pool = None;
        self
    }

    pub fn table(mut self, table: &str) -> Self {
        self.table = table.into();
        self
    }

    pub fn vector_dimensions(mut self, vector_dimensions: i32) -> Self {
        self.vector_dimensions = vector_dimensions;
        self
    }

    pub fn embedder<E: Embedder + 'static>(mut self, embedder: E) -> Self {
        self.embedder = Some(Arc::new(embedder));
        self
    }

    // Finalize the builder and construct the Store object
    pub async fn build(self) -> Result<Store, Box<dyn Error>> {
        if self.embedder.is_none() {
            return Err("Embedder is required".into());
        }

        Ok(Store {
            pool: self.get_pool().await?,
            table: self.table,
            vector_dimensions: self.vector_dimensions,
            embedder: self.embedder.unwrap(),
        })
    }

    async fn get_pool(&self) -> Result<Pool<Sqlite>, Box<dyn Error>> {
        match &self.pool {
            Some(pool) => Ok(pool.clone()),
            None => {
                let connection_url = self
                    .connection_url
                    .as_ref()
                    .ok_or("Connection URL or pool is required")?;

                let pool: Pool<Sqlite> = SqlitePoolOptions::new()
                    .connect_with(
                        SqliteConnectOptions::from_str(connection_url)?
                            .create_if_missing(true)
                            .extension("vec0"),
                    )
                    .await?;

                Ok(pool)
            }
        }
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
        let result = StoreBuilder::new()
            .connection_url(":memory:")
            // no .embedder()
            .build()
            .await;
        assert!(result.is_err());
        assert!(err_msg(result).contains("Embedder"));
    }

    #[tokio::test]
    async fn build_without_pool_or_url_returns_error() {
        let result = StoreBuilder::new()
            .embedder(DummyEmbedder)
            // no .connection_url() or .pool()
            .build()
            .await;
        assert!(result.is_err());
        let msg = err_msg(result);
        assert!(!msg.is_empty());
    }
}
