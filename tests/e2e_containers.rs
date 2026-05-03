/// Tier 3 — full-stack e2e tests using smolvm (no Docker required).
///
/// One shared VM is started per image for the entire test binary (pgvector tests share one
/// machine, qdrant tests share another). Machines are torn down when the process exits via
/// `Drop` on the global fixture. This avoids re-pulling images on every test.
///
/// Run: `cargo test --test e2e_containers --features postgres,qdrant`
///
/// Requires smolvm on PATH. Tests skip automatically if smolvm is unavailable.
mod common;

use std::net::TcpListener;
use std::sync::{Mutex, OnceLock};

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
    /// The host port the service is reachable on.
    pub port: u16,
}

impl SmolvmMachine {
    /// Create and start a named machine, returning an RAII guard that cleans up on drop.
    /// `extra_args` are forwarded to `machine create` (e.g. `-p HOST:GUEST`, `-e KEY=VAL`).
    fn start(name: &str, image: &str, extra_args: &[&str], port: u16) -> Self {
        // Step 1: create the named machine configuration.
        let mut create = std::process::Command::new("smolvm");
        create.args(["machine", "create", "--net", "--image", image]);
        for arg in extra_args {
            create.arg(arg);
        }
        create.arg(name);
        let status = create
            .status()
            .unwrap_or_else(|e| panic!("smolvm machine create failed: {e}"));
        assert!(status.success(), "smolvm machine create exited non-zero");

        // Step 2: start it in the background.
        let status = std::process::Command::new("smolvm")
            .args(["machine", "start", "-n", name])
            .status()
            .unwrap_or_else(|e| panic!("smolvm machine start failed: {e}"));
        assert!(status.success(), "smolvm machine start exited non-zero");

        Self {
            name: name.to_string(),
            port,
        }
    }
}

impl Drop for SmolvmMachine {
    fn drop(&mut self) {
        // best-effort cleanup — ignore errors
        let _ = std::process::Command::new("smolvm")
            .args(["machine", "stop", "-n", &self.name])
            .status();
        let _ = std::process::Command::new("smolvm")
            .args(["machine", "delete", "--force", &self.name])
            .status();
    }
}

/// Poll `addr` until a TCP connection succeeds or `timeout_secs` elapses.
/// Prints "waiting for <label>..." once, then a dot every 5 seconds.
async fn wait_for_tcp(addr: &str, label: &str, timeout_secs: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut last_dot = std::time::Instant::now();
    eprintln!("waiting for {label} at {addr}...");
    loop {
        if tokio::net::TcpStream::connect(addr).await.is_ok() {
            eprintln!(" ready.");
            return;
        }
        if std::time::Instant::now() > deadline {
            panic!("{label} did not become ready within {timeout_secs}s on {addr}");
        }
        if last_dot.elapsed() >= std::time::Duration::from_secs(5) {
            eprint!(".");
            last_dot = std::time::Instant::now();
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
}

/// Global fixture for the pgvector machine. Started once, lives until process exit.
static PGVECTOR_MACHINE: OnceLock<Mutex<Option<SmolvmMachine>>> = OnceLock::new();
/// Global fixture for the qdrant machine. Started once, lives until process exit.
static QDRANT_MACHINE: OnceLock<Mutex<Option<SmolvmMachine>>> = OnceLock::new();

/// Start (or return the already-running) pgvector machine. Returns the host port.
async fn pgvector_port() -> u16 {
    let cell = PGVECTOR_MACHINE.get_or_init(|| {
        let port = free_port();
        let port_map = format!("{port}:5432");
        let name = format!("langchainx-pg-shared");
        let machine = SmolvmMachine::start(
            &name,
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
            port,
        );
        Mutex::new(Some(machine))
    });

    let port = cell.lock().unwrap().as_ref().unwrap().port;
    wait_for_tcp(&format!("127.0.0.1:{port}"), "postgres", 90).await;
    // Extra settle time for Postgres init after TCP opens
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    port
}

/// Start (or return the already-running) qdrant machine. Returns the host gRPC port.
async fn qdrant_port() -> u16 {
    let cell = QDRANT_MACHINE.get_or_init(|| {
        let port = free_port();
        let port_map = format!("{port}:6334");
        let name = format!("langchainx-qdrant-shared");
        let machine = SmolvmMachine::start(&name, "qdrant/qdrant:latest", &["-p", &port_map], port);
        Mutex::new(Some(machine))
    });

    let port = cell.lock().unwrap().as_ref().unwrap().port;
    wait_for_tcp(&format!("127.0.0.1:{port}"), "qdrant", 90).await;
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    port
}

// ---------------------------------------------------------------------------
// Postgres / pgvector
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
mod pgvector_tests {
    use langchainx::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{pgvector::StoreBuilder, VectorStore},
    };

    use crate::common::FakeEmbedder;
    use crate::{pgvector_port, smolvm_available};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_pgvector_add_and_search() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let port = pgvector_port().await;
        let url = format!("postgresql://test:test@127.0.0.1:{port}/testdb");

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
    #[serial]
    async fn test_pgvector_empty_collection_search() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let port = pgvector_port().await;
        let url = format!("postgresql://test:test@127.0.0.1:{port}/testdb");

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
    use langchainx::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{qdrant::StoreBuilder, VectorStore},
    };
    use qdrant_client::Qdrant;

    use crate::common::FakeEmbedder;
    use crate::{qdrant_port, smolvm_available};
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_qdrant_add_and_search() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let grpc_port = qdrant_port().await;
        let url = format!("http://127.0.0.1:{grpc_port}");

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
    #[serial]
    async fn test_qdrant_search_limit_respected() {
        if !smolvm_available() {
            eprintln!("SKIP: smolvm not available");
            return;
        }

        let grpc_port = qdrant_port().await;
        let url = format!("http://127.0.0.1:{grpc_port}");

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
