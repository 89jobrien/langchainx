//! OpenSearch k-NN index management and vector search.
use async_trait::async_trait;
use opensearch::http::request::JsonBody;
use opensearch::http::response::Response;
use opensearch::indices::{IndicesCreateParts, IndicesDeleteParts};
use opensearch::{BulkParts, SearchParts};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

pub use opensearch::OpenSearch;
pub use opensearch::auth::Credentials;
pub use opensearch::cert::CertificateValidation;
pub use opensearch::http::transport::{SingleNodeConnectionPool, TransportBuilder};

use langchainx_embedding::embedding::embedder_trait::Embedder;
use langchainx_embedding::schemas::Document;

use crate::{VecStoreOptions, VectorStore, VectorStoreError};

/// Vector store backed by an OpenSearch k-NN index.
pub struct Store {
    /// OpenSearch client used for index operations.
    pub client: OpenSearch,
    /// Default embedder for documents and queries.
    pub embedder: Arc<dyn Embedder>,
    /// Neighbor count sent to k-NN queries.
    pub k: i32,
    /// OpenSearch index name.
    pub index: String,
    /// Index field containing embedding vectors.
    pub vector_field: String,
    /// Index field containing document text.
    pub content_field: String,
}

// https://opensearch.org/docs/latest/search-plugins/knn/approximate-knn/
// https://opensearch.org/blog/efficient-filters-in-knn/
// https://opensearch.org/docs/latest/clients/rust/

impl Store {
    /// Deletes the configured OpenSearch index.
    pub async fn delete_index(&self) -> Result<Response, VectorStoreError> {
        let response = self
            .client
            .indices()
            .delete(IndicesDeleteParts::Index(&[&self.index]))
            .send()
            .await?;

        let result = response
            .error_for_status_code()
            .map_err(|e| VectorStoreError::ConnectionError(e.to_string()))?;

        Ok(result)
    }

    /// Creates the configured 1,536-dimension FAISS HNSW index.
    pub async fn create_index(&self) -> Result<Response, VectorStoreError> {
        let body = json!({
            "settings": {
                "index.knn": true,
                "knn.algo_param": {
                    "ef_search": "512"
                },
            },
            "mappings": {
                "properties": {
                    &self.vector_field: {
                        "type": "knn_vector",
                        "dimension": 1536,
                        "method": {
                            "engine": "faiss",
                            "name": "hnsw",
                            "space_type": "l2",
                            "parameters": {
                                "ef_construction": 512,
                                "m": 16
                            }
                        }
                    },
                    &self.content_field: {
                        "type": "text"
                    },
                    "metadata": {
                        "properties": {
                            "source": {
                                "type": "text",
                            }
                        }
                    }
                }
            }
        });

        let response = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(&self.index))
            .body(body)
            .send()
            .await?;

        let result = response
            .error_for_status_code()
            .map_err(|e| VectorStoreError::ConnectionError(e.to_string()))?;

        Ok(result)
    }
}

#[async_trait]
impl VectorStore for Store {
    type Options = VecStoreOptions<Value>;

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

        let mut body: Vec<JsonBody<_>> = Vec::with_capacity(docs.len() * 2);

        for (doc, vector) in docs.iter().zip(vectors.iter()) {
            let operation = json!({"index": {}});
            body.push(operation.into());

            let document = json!({
                &self.content_field: doc.page_content,
                "metadata": doc.metadata,
                &self.vector_field: vector,
            });
            body.push(document.into());
        }

        let response = self
            .client
            .bulk(BulkParts::Index(&self.index))
            .body(body)
            .send()
            .await?
            .error_for_status_code()
            .map_err(|e| VectorStoreError::ConnectionError(e.to_string()))?;

        let response_body = response.json::<Value>().await?;

        let ids = response_body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| serde_json::from_value::<String>(item["index"]["_id"].clone()).unwrap())
            .collect::<Vec<_>>();

        Ok(ids)
    }

    async fn similarity_search(
        &self,
        query: &str,
        limit: usize,
        opt: &Self::Options,
    ) -> Result<Vec<Document>, VectorStoreError> {
        let query_vector = self.embedder.embed_query(query).await?;
        let query = build_similarity_search_query(
            query_vector,
            &self.vector_field,
            limit,
            self.k,
            opt.filters.clone(),
        );
        let request_limit = i64::try_from(limit).map_err(|_| {
            VectorStoreError::OtherError(format!("search limit {limit} exceeds i64::MAX"))
        })?;

        let response = self
            .client
            .search(SearchParts::Index(&[&self.index]))
            .from(0)
            .size(request_limit)
            .body(query)
            .send()
            .await?
            .error_for_status_code()
            .map_err(|e| VectorStoreError::ConnectionError(e.to_string()))?;

        let response_body = response.json::<Value>().await?;

        let aoss_documents = response_body["hits"]["hits"]
            .as_array()
            .ok_or_else(|| {
                VectorStoreError::OtherError(
                    "OpenSearch response is missing hits.hits array".to_string(),
                )
            })?
            .iter()
            .map(|raw_value| serde_json::from_value::<HashMap<String, Value>>(raw_value.clone()))
            .collect::<Result<Vec<_>, _>>()?;

        let documents = aoss_documents
            .into_iter()
            .map(|item| -> Result<Document, VectorStoreError> {
                let page_content =
                    serde_json::from_value::<String>(item["_source"][&self.content_field].clone())?;
                let metadata = serde_json::from_value::<HashMap<String, Value>>(
                    item["_source"]["metadata"].clone(),
                )?;
                let score = serde_json::from_value::<f64>(item["_score"].clone())?;
                Ok(Document {
                    page_content,
                    metadata,
                    score,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(documents)
    }
}

fn build_similarity_search_query(
    embedded_query: Vec<f64>,
    vector_field: &str,
    size: usize,
    k: i32,
    maybe_filter: Option<Value>,
) -> Value {
    match maybe_filter {
        Some(filter) => {
            json!({
              "size": size,
              "query": {
                "knn": {
                  vector_field: {
                    "vector": embedded_query,
                    "k": k,
                    "filter": filter,
                  }
                }
              }
            })
        }
        None => {
            json!({
              "size": size,
              "query": {
                "knn": {
                  vector_field: {
                    "vector": embedded_query,
                    "k": k,
                  }
                }
              }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use langchainx_embedding::embedding::EmbedderError;
    use opensearch::http::transport::Transport;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;

    #[derive(Debug)]
    struct FakeEmbedder;

    #[async_trait]
    impl Embedder for FakeEmbedder {
        async fn embed_documents(
            &self,
            documents: &[String],
        ) -> Result<Vec<Vec<f64>>, EmbedderError> {
            Ok(documents.iter().map(|_| vec![0.0]).collect())
        }

        async fn embed_query(&self, _text: &str) -> Result<Vec<f64>, EmbedderError> {
            Ok(vec![0.0])
        }
    }

    #[tokio::test]
    async fn similarity_search_sends_requested_limit() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buffer = vec![0; 8192];
            let read = socket.read(&mut buffer).await.unwrap();
            request_tx
                .send(String::from_utf8_lossy(&buffer[..read]).into_owned())
                .unwrap();
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 20\r\n\r\n{\"hits\":{\"hits\":[]}}",
                )
                .await
                .unwrap();
        });

        let transport = Transport::single_node(&format!("http://{address}")).unwrap();
        let store = Store {
            client: OpenSearch::new(transport),
            embedder: Arc::new(FakeEmbedder),
            k: 10,
            index: "test".to_string(),
            vector_field: "vector".to_string(),
            content_field: "content".to_string(),
        };

        store
            .similarity_search("query", 7, &VecStoreOptions::default())
            .await
            .unwrap();
        let request = request_rx.await.unwrap();

        assert!(request.lines().next().unwrap().contains("size=7"));
    }
}
