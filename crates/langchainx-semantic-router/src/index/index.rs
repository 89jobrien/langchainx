//! Storage and similarity-query interface for semantic routes.
use async_trait::async_trait;

use crate::{IndexError, Router};

#[async_trait]
/// Stores routes and searches their embedding vectors.
pub trait Index: Send + Sync {
    /// Adds or replaces routes in the index.
    async fn add(&mut self, router: &[Router]) -> Result<(), IndexError>;

    /// Deletes a route by name.
    async fn delete(&mut self, route_name: &str) -> Result<(), IndexError>;

    /// Returns up to `top_k` route names and similarity scores for a query vector.
    async fn query(&self, vector: &[f64], top_k: usize) -> Result<Vec<(String, f64)>, IndexError>;

    /// Returns all indexed routes.
    async fn get_routers(&self) -> Result<Vec<Router>, IndexError>;

    /// Returns one indexed route by name.
    async fn get_router(&self, route_name: &str) -> Result<Router, IndexError>;

    /// Removes all routes from the index.
    async fn delete_index(&mut self) -> Result<(), IndexError>;
}
