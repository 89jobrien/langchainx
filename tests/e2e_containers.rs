/// Tier 3 — full-stack e2e tests using smolvm (no Docker required).
///
/// Images are pre-packed into `.smolmachine` artifacts on first run and cached at
/// `SMOLVM_ARTIFACT_DIR` (default: `/tmp/langchainx-smolmachines/`). Subsequent runs
/// boot from the artifact in ~250ms with no pull. Tests are `#[serial]` so only one
/// machine per service runs at a time.
///
/// Future (plan C): set `SMOLVM_ARTIFACT_DIR` to `.cache/smolmachines/` in the repo
/// and commit the artifacts — CI will get zero-pull cold starts.
///
/// Run: `cargo test --test e2e_containers --features postgres,qdrant`
///
/// Requires smolvm on PATH. Tests skip automatically if smolvm is unavailable.
mod common;

use std::net::TcpListener;
use std::path::{Path, PathBuf};

/// Directory where packed `.smolmachine` artifacts are cached.
/// Override with `SMOLVM_ARTIFACT_DIR` env var for plan-C committed artifacts.
fn artifact_dir() -> PathBuf {
    std::env::var("SMOLVM_ARTIFACT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/langchainx-smolmachines"))
}

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

/// Pack `image` into a `.smolmachine` artifact at `artifact_path` if not already present.
/// Returns the artifact path. On first call per image this pulls and packs; subsequent
/// calls return immediately (~0ms).
fn smolvm_pack_or_reuse(image: &str, artifact_path: &Path) {
    if artifact_path.exists() {
        eprintln!("smolvm artifact cache hit: {}", artifact_path.display());
        return;
    }
    std::fs::create_dir_all(artifact_path.parent().unwrap())
        .expect("failed to create artifact dir");
    eprintln!(
        "smolvm packing image {image} -> {}",
        artifact_path.display()
    );
    let status = std::process::Command::new("smolvm")
        .args(["pack", "create", "--image", image, "-o"])
        .arg(artifact_path.with_extension("")) // smolvm appends .smolmachine itself
        .status()
        .unwrap_or_else(|e| panic!("smolvm pack create failed: {e}"));
    assert!(status.success(), "smolvm pack create exited non-zero");
    assert!(
        artifact_path.exists(),
        "expected artifact at {} after pack",
        artifact_path.display()
    );
}

/// RAII guard: stops and deletes the named smolvm machine on drop.
#[allow(dead_code)]
struct SmolvmMachine {
    pub name: String,
    pub port: u16,
}

impl SmolvmMachine {
    /// Create and start a persistent machine from a pre-packed artifact.
    /// `extra_args` are forwarded to `machine create` (e.g. `-p HOST:GUEST`, `-e KEY=VAL`).
    fn launch(name: &str, artifact_path: &Path, extra_args: &[&str], port: u16) -> Self {
        // Create from artifact (fast boot, no pull).
        let mut create = std::process::Command::new("smolvm");
        create.args(["machine", "create", name, "--net", "--from"]);
        create.arg(artifact_path);
        for arg in extra_args {
            create.arg(arg);
        }
        let status = create
            .status()
            .unwrap_or_else(|e| panic!("smolvm machine create failed: {e}"));
        assert!(status.success(), "smolvm machine create exited non-zero");

        // Start the machine.
        let status = std::process::Command::new("smolvm")
            .args(["machine", "start", "--name", name])
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
        let _ = std::process::Command::new("smolvm")
            .args(["machine", "stop", "--name", &self.name])
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

// ---------------------------------------------------------------------------
// Postgres / pgvector
// ---------------------------------------------------------------------------

#[cfg(feature = "postgres")]
mod pgvector_tests {
    use langchainx::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{VectorStore, pgvector::StoreBuilder},
    };

    use crate::common::FakeEmbedder;
    use crate::{SmolvmMachine, free_port, smolvm_available, wait_for_tcp};
    use serial_test::serial;

    async fn start_pgvector() -> (SmolvmMachine, String) {
        let artifact = crate::artifact_dir().join("pgvector-pg16.smolmachine");
        crate::smolvm_pack_or_reuse("pgvector/pgvector:pg16", &artifact);
        let port = free_port();
        let port_map = format!("{port}:5432");
        let machine = SmolvmMachine::launch(
            "pgvector-test",
            &artifact,
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
        wait_for_tcp(&format!("127.0.0.1:{port}"), "postgres", 90).await;
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        let url = format!("postgresql://test:test@127.0.0.1:{port}/testdb");
        (machine, url)
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    use langchainx::{
        add_documents,
        schemas::Document,
        similarity_search,
        vectorstore::{VectorStore, qdrant::StoreBuilder},
    };
    use qdrant_client::Qdrant;

    use crate::common::FakeEmbedder;
    use crate::{SmolvmMachine, free_port, smolvm_available, wait_for_tcp};
    use serial_test::serial;

    async fn start_qdrant() -> (SmolvmMachine, String) {
        let artifact = crate::artifact_dir().join("qdrant-latest.smolmachine");
        crate::smolvm_pack_or_reuse("qdrant/qdrant:latest", &artifact);
        let port = free_port();
        let port_map = format!("{port}:6334");
        let machine = SmolvmMachine::launch("qdrant-test", &artifact, &["-p", &port_map], port);
        wait_for_tcp(&format!("127.0.0.1:{port}"), "qdrant", 90).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let url = format!("http://127.0.0.1:{port}");
        (machine, url)
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
