use std::collections::HashMap;

use async_trait::async_trait;

use crate::{IndexError, Router, utils::cosine_similarity};

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

impl Default for MemoryIndex {
    fn default() -> Self {
        Self::new()
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

    fn make_router(name: &str, vecs: Vec<Vec<f64>>) -> Router {
        Router::new(name, &["utterance"]).with_embedding(vecs)
    }

    #[tokio::test]
    async fn test_add_and_get_router() {
        let mut index = MemoryIndex::new();
        let router = make_router("greet", vec![vec![1.0, 0.0, 0.0]]);
        index.add(&[router]).await.unwrap();

        let retrieved = index.get_router("greet").await.unwrap();
        assert_eq!(retrieved.name, "greet");
    }

    #[tokio::test]
    async fn test_add_missing_embedding_errors() {
        let mut index = MemoryIndex::new();
        let router = Router::new("bare", &["utterance"]); // no embedding
        let result = index.add(&[router]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_router() {
        let mut index = MemoryIndex::new();
        let router = make_router("greet", vec![vec![1.0, 0.0, 0.0]]);
        index.add(&[router]).await.unwrap();
        index.delete("greet").await.unwrap();
        let result = index.get_router("greet").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_returns_most_similar() {
        let mut index = MemoryIndex::new();
        // "greet" aligns with [1,0,0]; "weather" aligns with [0,0,1]
        let greet = make_router("greet", vec![vec![1.0, 0.0, 0.0]]);
        let weather = make_router("weather", vec![vec![0.0, 0.0, 1.0]]);
        index.add(&[greet, weather]).await.unwrap();

        let results = index.query(&[1.0, 0.0, 0.0], 2).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].0, "greet");
    }

    #[tokio::test]
    async fn test_get_routers_all() {
        let mut index = MemoryIndex::new();
        index
            .add(&[make_router("a", vec![vec![1.0, 0.0]])])
            .await
            .unwrap();
        index
            .add(&[make_router("b", vec![vec![0.0, 1.0]])])
            .await
            .unwrap();
        let all = index.get_routers().await.unwrap();
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_index_clears_all() {
        let mut index = MemoryIndex::new();
        index
            .add(&[make_router("a", vec![vec![1.0, 0.0]])])
            .await
            .unwrap();
        index.delete_index().await.unwrap();
        let all = index.get_routers().await.unwrap();
        assert!(all.is_empty());
    }
}
