/// Tier 3 — full-stack e2e tests using smolvm (no Docker required).
///
/// Spins up real Postgres/pgvector and Qdrant VMs via smolvm, runs
/// add→search round-trips with FakeEmbedder, then tears them down.
///
/// Run: `cargo test --test e2e_containers --features postgres,qdrant`
///
/// Requires smolvm on PATH. Tests skip automatically if smolvm is unavailable.
mod common;

use std::net::TcpListener;

/// Allocate an available localhost port by binding to :0 and releasing it.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind")
        .local_addr()
        .unwrap()
        .port()
}

/// Returns true if `smolvm` is on PATH.
fn smolvm_available() -> bool {
    std::process::Command::new("smolvm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// RAII guard: stops and deletes a named smolvm machine on drop.
struct SmolvmMachine {
    name: String,
}

impl SmolvmMachine {
    /// Start a machine in detached mode and return a guard.
    /// `extra_args` are inserted before `--image` (e.g. `-p HOST:GUEST`, `-e KEY=VAL`).
    fn start(name: &str, image: &str, extra_args: &[&str]) -> Self {
        let mut cmd = std::process::Command::new("smolvm");
        cmd.args(["machine", "run", "--detach", "--net"]);
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd.args(["--image", image]);

        let status = cmd
            .status()
            .unwrap_or_else(|e| panic!("smolvm machine run failed: {e}"));
        assert!(status.success(), "smolvm machine run exited non-zero");

        Self {
            name: name.to_string(),
        }
    }
}

impl Drop for SmolvmMachine {
    fn drop(&mut self) {
        // best-effort cleanup
        let _ = std::process::Command::new("smolvm")
            .args(["machine", "stop", "--name", &self.name])
            .status();
        let _ = std::process::Command::new("smolvm")
            .args(["machine", "delete", &self.name])
            .status();
    }
}

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

    use crate::common::FakeEmbedder;
    use crate::{free_port, smolvm_available, SmolvmMachine};

    async fn start_pgvector() -> (SmolvmMachine, String) {
        let port = free_port();
        let port_map = format!("{}:5432", port);

        let machine_name = format!("langchainx-test-pg-{port}");

        // smolvm machine run --detach --net -p PORT:5432
        //   -e POSTGRES_USER=test -e POSTGRES_PASSWORD=test -e POSTGRES_DB=testdb
        //   --image pgvector/pgvector:pg16
        let _m = SmolvmMachine::start(
            &machine_name,
            "pgvector/pgvector:pg16",
            &[
                "-p",
                &port_map,
                "-e",
                "POSTGRES_USER=test",
                "-e",
                "POSTGRES_PASSWORD=test",
                "-e",
                "POSTGRES_DB=testdb",
            ],
        );

        // Wait for Postgres to become ready
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let result = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}")).await;
            if result.is_ok() {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("Postgres did not become ready within 30s on port {port}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        // Extra settle time for Postgres to finish initialising after TCP opens
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let url = format!("postgresql://test:test@127.0.0.1:{port}/testdb");
        (_m, url)
    }

    #[tokio::test]
    async fn test_pgvector_add_and_search() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let (_machine, url) = start_pgvector().await;

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
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let (_machine, url) = start_pgvector().await;

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

    use crate::common::FakeEmbedder;
    use crate::{free_port, smolvm_available, SmolvmMachine};

    async fn start_qdrant() -> (SmolvmMachine, String) {
        let grpc_port = free_port();
        let port_map = format!("{}:6334", grpc_port);
        let machine_name = format!("langchainx-test-qdrant-{grpc_port}");

        let _m = SmolvmMachine::start(&machine_name, "qdrant/qdrant:latest", &["-p", &port_map]);

        // Wait for Qdrant gRPC to become ready
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let result = tokio::net::TcpStream::connect(format!("127.0.0.1:{grpc_port}")).await;
            if result.is_ok() {
                break;
            }
            if std::time::Instant::now() > deadline {
                panic!("Qdrant did not become ready within 30s on port {grpc_port}");
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let url = format!("http://127.0.0.1:{grpc_port}");
        (_m, url)
    }

    #[tokio::test]
    async fn test_qdrant_add_and_search() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let (_machine, url) = start_qdrant().await;

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
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let (_machine, url) = start_qdrant().await;

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
