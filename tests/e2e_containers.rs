/// Tier 3 — full-stack e2e tests using testcontainers.
///
/// Requires Docker. Spins up real Postgres/pgvector and Qdrant containers,
/// runs add→search round-trips with FakeEmbedder, then tears down automatically.
///
/// Run: `cargo test --test e2e_containers --features postgres,qdrant`
mod common;

// ---------------------------------------------------------------------------
// Postgres / pgvector
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
mod pgvector_tests {
    use langchain_rust::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{pgvector::StoreBuilder, VectorStore},
    };
    use testcontainers::{
        core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt,
    };

    use crate::common::FakeEmbedder;

    async fn start_pgvector() -> (ContainerAsync<GenericImage>, String) {
        let container = GenericImage::new("pgvector/pgvector", "pg16")
            .with_wait_for(WaitFor::message_on_stderr(
                "database system is ready to accept connections",
            ))
            .with_env_var("POSTGRES_USER", "test")
            .with_env_var("POSTGRES_PASSWORD", "test")
            .with_env_var("POSTGRES_DB", "testdb")
            .start()
            .await
            .expect("failed to start pgvector container");

        // publish_all_ports is true by default when no explicit mapping is given
        let port = container.get_host_port_ipv4(5432).await.unwrap();
        let url = format!("postgresql://test:test@localhost:{port}/testdb");
        (container, url)
    }

    #[tokio::test]
    async fn test_pgvector_add_and_search() {
        let (_container, url) = start_pgvector().await;

        let embedder = FakeEmbedder::new(3);

        let store = StoreBuilder::new()
            .embedder(embedder)
            .connection_url(&url)
            .vector_dimensions(3)
            .pre_delete_collection(true)
            .build()
            .await
            .expect("failed to build pgvector store");

        let docs = vec![
            Document::new("rust is a systems programming language"),
            Document::new("python is used for data science"),
            Document::new("cargo is the rust package manager"),
        ];

        add_documents!(store, &docs)
            .await
            .expect("failed to add documents");

        let results = similarity_search!(store, "rust package manager", 2)
            .await
            .expect("similarity search failed");

        assert!(!results.is_empty(), "search returned no results");
        assert!(results.len() <= 2, "returned more results than requested");
    }

    #[tokio::test]
    async fn test_pgvector_empty_collection_search() {
        let (_container, url) = start_pgvector().await;

        let embedder = FakeEmbedder::new(3);

        let store = StoreBuilder::new()
            .embedder(embedder)
            .connection_url(&url)
            .vector_dimensions(3)
            .pre_delete_collection(true)
            .collection_name("empty_test")
            .build()
            .await
            .expect("failed to build pgvector store");

        let results = similarity_search!(store, "anything", 5)
            .await
            .expect("search on empty collection failed");

        assert!(
            results.is_empty(),
            "expected 0 results from empty collection"
        );
    }
}

// ---------------------------------------------------------------------------
// Qdrant
// ---------------------------------------------------------------------------

#[cfg(feature = "qdrant")]
mod qdrant_tests {
    use langchain_rust::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{qdrant::StoreBuilder, VectorStore},
    };
    use qdrant_client::Qdrant;
    use testcontainers::{
        core::WaitFor, runners::AsyncRunner, ContainerAsync, GenericImage, ImageExt,
    };

    use crate::common::FakeEmbedder;

    async fn start_qdrant() -> (ContainerAsync<GenericImage>, String) {
        let container = GenericImage::new("qdrant/qdrant", "latest")
            .with_wait_for(WaitFor::message_on_stdout("gRPC server listening on"))
            .start()
            .await
            .expect("failed to start qdrant container");

        let port = container.get_host_port_ipv4(6334).await.unwrap();
        let url = format!("http://localhost:{port}");
        (container, url)
    }

    #[tokio::test]
    async fn test_qdrant_add_and_search() {
        let (_container, url) = start_qdrant().await;

        let embedder = FakeEmbedder::new(4);

        let client = Qdrant::from_url(&url)
            .build()
            .expect("failed to build Qdrant client");

        let store = StoreBuilder::new()
            .client(client)
            .embedder(embedder)
            .collection_name("test_collection")
            .recreate_collection(true)
            .build()
            .await
            .expect("failed to build qdrant store");

        let docs = vec![
            Document::new("the quick brown fox"),
            Document::new("jumps over the lazy dog"),
            Document::new("rust ownership and borrowing"),
        ];

        add_documents!(store, &docs)
            .await
            .expect("failed to add documents");

        let results = similarity_search!(store, "fox jumps", 2)
            .await
            .expect("similarity search failed");

        assert!(!results.is_empty(), "search returned no results");
        assert!(results.len() <= 2, "returned more results than requested");
    }

    #[tokio::test]
    async fn test_qdrant_search_limit_respected() {
        let (_container, url) = start_qdrant().await;

        let embedder = FakeEmbedder::new(4);

        let client = Qdrant::from_url(&url)
            .build()
            .expect("failed to build Qdrant client");

        let store = StoreBuilder::new()
            .client(client)
            .embedder(embedder)
            .collection_name("limit_test")
            .recreate_collection(true)
            .build()
            .await
            .expect("failed to build qdrant store");

        let docs: Vec<Document> = (0..10)
            .map(|i| Document::new(format!("document number {i}")))
            .collect();

        add_documents!(store, &docs)
            .await
            .expect("failed to add documents");

        let results = similarity_search!(store, "document", 3)
            .await
            .expect("similarity search failed");

        assert!(
            results.len() <= 3,
            "returned {} results, expected at most 3",
            results.len()
        );
    }
}
