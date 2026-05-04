#![allow(clippy::module_inception)]

mod error;
mod options;
mod vectorstore;

#[cfg(feature = "postgres")]
pub mod pgvector;

#[cfg(feature = "sqlite-vss")]
pub mod sqlite_vss;

#[cfg(feature = "sqlite-vec")]
pub mod sqlite_vec;

#[cfg(feature = "surrealdb")]
pub mod surrealdb;

#[cfg(feature = "opensearch")]
pub mod opensearch;

#[cfg(feature = "qdrant")]
pub mod qdrant;

pub use error::VectorStoreError;
pub use options::*;
pub use vectorstore::*;
