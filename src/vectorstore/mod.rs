// Thin re-exports from langchainx-vectorstore crate.
// All vectorstore logic lives in crates/langchainx-vectorstore.

#[cfg(feature = "postgres")]
pub mod pgvector {
    pub use langchainx_vectorstore::pgvector::*;
}

#[cfg(feature = "sqlite-vss")]
pub mod sqlite_vss {
    pub use langchainx_vectorstore::sqlite_vss::*;
}

#[cfg(feature = "sqlite-vec")]
pub mod sqlite_vec {
    pub use langchainx_vectorstore::sqlite_vec::*;
}

#[cfg(feature = "surrealdb")]
pub mod surrealdb {
    pub use langchainx_vectorstore::surrealdb::*;
}

#[cfg(feature = "opensearch")]
pub mod opensearch {
    pub use langchainx_vectorstore::opensearch::*;
}

#[cfg(feature = "qdrant")]
pub mod qdrant {
    pub use langchainx_vectorstore::qdrant::*;
}

pub use langchainx_vectorstore::{Retriever, VecStoreOptions, VectorStore, VectorStoreError};
