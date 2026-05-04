use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};

use langchainx_embedding::embedding::embedder_trait::Embedder;
use langchainx_embedding::schemas::Document;

use crate::{VecStoreOptions, VectorStore, VectorStoreError};

pub struct Store {
    pub pool: Pool<Sqlite>,
    pub(crate) table: String,
    pub(crate) vector_dimensions: i32,
    pub(crate) embedder: Arc<dyn Embedder>,
}

pub type SqliteOptions = VecStoreOptions<Value>;

impl Store {
    pub async fn initialize(&self) -> Result<(), VectorStoreError> {
        self.create_table_if_not_exists().await?;
        Ok(())
    }

    async fn create_table_if_not_exists(&self) -> Result<(), VectorStoreError> {
        if self.vector_dimensions <= 0 {
            return Err(VectorStoreError::OtherError(
                "vector_dimensions must be greater than zero".to_string(),
            ));
        }

        let table = quoted_identifier(&self.table)?;
        let vector_table = quoted_identifier(&format!("vec_{}", self.table))?;
        let trigger = quoted_identifier(&format!("embed_text_{}", self.table))?;

        sqlx::query(&format!(
            r#"
                CREATE TABLE IF NOT EXISTS {table}
                (
                  rowid INTEGER PRIMARY KEY AUTOINCREMENT,
                  text TEXT NOT NULL,
                  metadata TEXT NOT NULL,
                  text_embedding TEXT NOT NULL
                )
                ;
                "#
        ))
        .execute(&self.pool)
        .await?;

        let dimensions = self.vector_dimensions;
        sqlx::query(&format!(
            r#"
                CREATE VIRTUAL TABLE IF NOT EXISTS {vector_table} USING vec0(
                  text_embedding float[{dimensions}]
                );
                "#
        ))
        .execute(&self.pool)
        .await?;

        // NOTE: python langchain seems to only use "embed_text" as the trigger name
        sqlx::query(&format!(
            r#"
                CREATE TRIGGER IF NOT EXISTS {trigger}
                AFTER INSERT ON {table}
                BEGIN
                    INSERT INTO {vector_table}(rowid, text_embedding)
                    VALUES (new.rowid, new.text_embedding)
                    ;
                END;
                "#
        ))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    fn get_filters(&self, opt: &SqliteOptions) -> Result<HashMap<String, Value>, VectorStoreError> {
        match &opt.filters {
            Some(Value::Object(map)) => {
                let filters = map.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                Ok(filters)
            }
            None => Ok(HashMap::new()),
            _ => Err(VectorStoreError::OtherError(
                "Invalid filters format".to_string(),
            )),
        }
    }
}

fn quoted_identifier(identifier: &str) -> Result<String, VectorStoreError> {
    let mut chars = identifier.chars();
    let Some(first) = chars.next() else {
        return Err(VectorStoreError::OtherError(
            "SQLite identifier cannot be empty".to_string(),
        ));
    };

    if !(first == '_' || first.is_ascii_alphabetic())
        || !chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    {
        return Err(VectorStoreError::OtherError(format!(
            "Invalid SQLite identifier: {identifier}"
        )));
    }

    Ok(format!("\"{identifier}\""))
}

fn metadata_filter_clause(filters: &HashMap<String, Value>) -> String {
    if filters.is_empty() {
        return "TRUE".to_string();
    }

    std::iter::repeat_n(
        "json_extract(e.metadata, ?) = json_extract(?, '$')",
        filters.len(),
    )
    .collect::<Vec<_>>()
    .join(" AND ")
}

fn validate_vector_dimensions(vector: &[f64], expected: i32) -> Result<(), VectorStoreError> {
    if expected <= 0 {
        return Err(VectorStoreError::OtherError(
            "vector_dimensions must be greater than zero".to_string(),
        ));
    }

    if vector.len() != expected as usize {
        return Err(VectorStoreError::OtherError(format!(
            "Embedding dimension mismatch: expected {expected}, got {}",
            vector.len()
        )));
    }

    Ok(())
}

#[async_trait]
impl VectorStore for Store {
    type Options = SqliteOptions;

    async fn add_documents(
        &self,
        docs: &[Document],
        opt: &Self::Options,
    ) -> Result<Vec<String>, VectorStoreError> {
        let texts: Vec<String> = docs.iter().map(|d| d.page_content.clone()).collect();

        let embedder = opt.embedder.as_ref().unwrap_or(&self.embedder);

        let vectors = embedder.embed_documents(&texts).await?;
        if vectors.len() != docs.len() {
            return Err(VectorStoreError::OtherError(
                "Number of vectors and documents do not match".to_string(),
            ));
        }

        let table = quoted_identifier(&self.table)?;

        let mut tx = self.pool.begin().await?;

        let mut ids = Vec::with_capacity(docs.len());

        for (doc, vector) in docs.iter().zip(vectors.iter()) {
            validate_vector_dimensions(vector, self.vector_dimensions)?;
            let metadata = serde_json::to_string(&doc.metadata)?;
            let text_embedding = serde_json::to_string(vector)?;
            let id = sqlx::query(&format!(
                r#"
                    INSERT INTO {table}
                        (text, metadata, text_embedding)
                    VALUES
                        (?,?,?)"#
            ))
            .bind(&doc.page_content)
            .bind(metadata)
            .bind(text_embedding)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();

            ids.push(id.to_string());
        }

        tx.commit().await?;

        Ok(ids)
    }

    async fn similarity_search(
        &self,
        query: &str,
        limit: usize,
        opt: &Self::Options,
    ) -> Result<Vec<Document>, VectorStoreError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let table = quoted_identifier(&self.table)?;
        let vector_table = quoted_identifier(&format!("vec_{}", self.table))?;
        let embedder = opt.embedder.as_ref().unwrap_or(&self.embedder);
        let query_vector = embedder.embed_query(query).await?;
        validate_vector_dimensions(&query_vector, self.vector_dimensions)?;
        let query_vector = serde_json::to_string(&query_vector)?;

        let filter = self.get_filters(opt)?;
        let metadata_query = metadata_filter_clause(&filter);

        let sql = format!(
            r#"SELECT
                    text,
                    metadata,
                    distance
                FROM {table} e
                INNER JOIN {vector_table} v on v.rowid = e.rowid
                WHERE v.text_embedding MATCH ? AND k = ? AND {metadata_query}
                ORDER BY distance
                LIMIT ?"#
        );
        let mut query = sqlx::query(&sql).bind(query_vector).bind(limit as i64);
        for (key, value) in filter {
            query = query
                .bind(format!("$.{key}"))
                .bind(serde_json::to_string(&value)?);
        }
        let rows = query.bind(limit as i64).fetch_all(&self.pool).await?;

        let docs = rows
            .into_iter()
            .map(|row| {
                let page_content: String = row.try_get("text")?;
                let metadata_json: String = row.try_get("metadata")?;
                let score: f64 = row.try_get("distance")?;
                let metadata_json = serde_json::from_str::<Value>(&metadata_json).map_err(|e| {
                    sqlx::Error::ColumnDecode {
                        index: "metadata".to_string(),
                        source: Box::new(e),
                    }
                })?;

                let metadata = if let Value::Object(obj) = metadata_json {
                    obj.into_iter().collect()
                } else {
                    HashMap::new()
                };

                Ok(Document {
                    page_content,
                    metadata,
                    score,
                })
            })
            .collect::<Result<Vec<Document>, sqlx::Error>>()
            .map_err(VectorStoreError::from)?;

        Ok(docs)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use langchainx_embedding::embedding::EmbedderError;
    use std::{collections::HashMap, sync::Arc};

    use serde_json::json;
    use sqlx::sqlite::SqlitePoolOptions;

    use super::*;

    struct StaticEmbedder {
        documents: Vec<Vec<f64>>,
        query: Vec<f64>,
    }

    #[async_trait]
    impl Embedder for StaticEmbedder {
        async fn embed_documents(
            &self,
            documents: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(self
                .documents
                .iter()
                .take(documents.len())
                .cloned()
                .collect())
        }

        async fn embed_query(&self, _text: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(self.query.clone())
        }
    }

    #[test]
    fn quoted_identifier_accepts_safe_names() {
        assert_eq!(quoted_identifier("documents_1").unwrap(), "\"documents_1\"");
    }

    #[test]
    fn quoted_identifier_rejects_unsafe_names() {
        assert!(quoted_identifier("").is_err());
        assert!(quoted_identifier("1documents").is_err());
        assert!(quoted_identifier("documents; DROP TABLE documents").is_err());
    }

    #[test]
    fn metadata_filter_clause_uses_bound_parameters() {
        let filters = HashMap::from([
            ("author".to_string(), json!("Alice")),
            ("year".to_string(), json!(2024)),
        ]);

        let clause = metadata_filter_clause(&filters);

        assert_eq!(
            clause,
            "json_extract(e.metadata, ?) = json_extract(?, '$') AND json_extract(e.metadata, ?) = json_extract(?, '$')"
        );
    }

    #[test]
    fn metadata_filter_clause_is_true_without_filters() {
        assert_eq!(metadata_filter_clause(&HashMap::new()), "TRUE");
    }

    #[test]
    fn validate_vector_dimensions_catches_mismatch() {
        let error = validate_vector_dimensions(&[1.0, 2.0], 3).unwrap_err();

        assert!(error.to_string().contains("expected 3, got 2"));
    }

    #[tokio::test]
    async fn add_documents_rejects_dimension_mismatch_before_insert() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = Store {
            pool,
            table: "documents".to_string(),
            vector_dimensions: 3,
            embedder: Arc::new(StaticEmbedder {
                documents: vec![vec![1.0, 2.0]],
                query: vec![1.0, 2.0, 3.0],
            }),
        };

        let error = store
            .add_documents(&[Document::new("hello")], &SqliteOptions::default())
            .await
            .unwrap_err();

        assert!(error.to_string().contains("expected 3, got 2"));
    }

    #[tokio::test]
    async fn similarity_search_zero_limit_returns_empty_without_vec_table() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        let store = Store {
            pool,
            table: "documents".to_string(),
            vector_dimensions: 3,
            embedder: Arc::new(StaticEmbedder {
                documents: vec![],
                query: vec![1.0, 2.0, 3.0],
            }),
        };

        let docs = store
            .similarity_search("hello", 0, &SqliteOptions::default())
            .await
            .unwrap();

        assert!(docs.is_empty());
    }
}
