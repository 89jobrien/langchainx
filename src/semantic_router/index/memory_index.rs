use std::collections::HashMap;

use async_trait::async_trait;

use crate::semantic_router::{utils::cosine_similarity, IndexError, Router};

use super::Index;

pub struct MemoryIndex {
    routers: HashMap<String, Router>,
}
impl MemoryIndex {
    pub fn new() -> Self {
        Self {
            routers: HashMap::new(),
        }
    }
}

#[async_trait]
impl Index for MemoryIndex {
    async fn add(&mut self, routers: &[Router]) -> Result<(), IndexError> {
        for router in routers {
            if router.embedding.is_none() {
                return Err(IndexError::MissingEmbedding(router.name.clone()));
            }
            if self.routers.contains_key(&router.name) {
                log::warn!("Router {} already exists in the index", router.name);
            }
            self.routers.insert(router.name.clone(), router.clone());
        }

        Ok(())
    }

    async fn delete(&mut self, router_name: &str) -> Result<(), IndexError> {
        if self.routers.remove(router_name).is_none() {
            log::warn!("Router {} not found in the index", router_name);
        }
        Ok(())
    }

    async fn query(&self, vector: &[f64], top_k: usize) -> Result<Vec<(String, f64)>, IndexError> {
        let mut all_similarities: Vec<(String, f64)> = Vec::new();

        // Compute similarity for each embedding of each router
        for (name, router) in &self.routers {
            if let Some(embeddings) = &router.embedding {
                for embedding in embeddings {
                    let similarity = cosine_similarity(vector, embedding);
                    all_similarities.push((name.clone(), similarity));
                }
            }
        }

        // Sort all similarities by descending similarity score
        all_similarities
            .sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Only keep the top_k similarities
        let top_similarities: Vec<(String, f64)> =
            all_similarities.into_iter().take(top_k).collect();

        Ok(top_similarities)
    }

    async fn get_routers(&self) -> Result<Vec<Router>, IndexError> {
        let routes = self.routers.values().cloned().collect();
        Ok(routes)
    }

    async fn get_router(&self, route_name: &str) -> Result<Router, IndexError> {
        return self
            .routers
            .get(route_name)
            .cloned()
            .ok_or(IndexError::RouterNotFound(route_name.into()));
    }

    async fn delete_index(&mut self) -> Result<(), IndexError> {
        self.routers.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic_router::Router;

    #[tokio::test]
    async fn test_add_and_query_router() {
        let mut index = MemoryIndex::new();
        // Route pointing along the x-axis
        let router = Router::new("x_route", &["hello"])
            .with_embedding(vec![vec![1.0, 0.0]]);
        index.add(&[router]).await.unwrap();

        // Query with the same direction → similarity = 1.0
        let results = index.query(&[1.0, 0.0], 1).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, "x_route");
        assert!((results[0].1 - 1.0).abs() < 1e-9);
    }

    #[tokio::test]
    async fn test_add_without_embedding_returns_error() {
        let mut index = MemoryIndex::new();
        let router = Router::new("no_embed", &["hello"]);
        let err = index.add(&[router]).await.unwrap_err();
        assert!(matches!(err, IndexError::MissingEmbedding(_)));
    }

    #[tokio::test]
    async fn test_query_returns_top_k() {
        let mut index = MemoryIndex::new();
        let r1 = Router::new("x_route", &["a"]).with_embedding(vec![vec![1.0, 0.0]]);
        let r2 = Router::new("y_route", &["b"]).with_embedding(vec![vec![0.0, 1.0]]);
        index.add(&[r1, r2]).await.unwrap();

        // Query along x → x_route should score higher
        let results = index.query(&[1.0, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, "x_route");
    }

    #[tokio::test]
    async fn test_delete_router() {
        let mut index = MemoryIndex::new();
        let router = Router::new("to_delete", &["x"]).with_embedding(vec![vec![1.0, 0.0]]);
        index.add(&[router]).await.unwrap();
        index.delete("to_delete").await.unwrap();
        let routers = index.get_routers().await.unwrap();
        assert!(routers.is_empty());
    }
}
