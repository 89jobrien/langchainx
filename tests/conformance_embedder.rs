/// Conformance tests for the `Embedder` trait contract.
///
/// Every `Embedder` impl must satisfy:
/// 1. `embed_query()` returns a non-empty vector.
/// 2. `embed_documents()` returns one vector per document.
/// 3. All vectors have the same dimensionality.
/// 4. Vectors are not all zeros (meaningful embeddings).
/// 5. Same input produces same output (deterministic).
mod common;

use langchainx::embedding::embedder_trait::Embedder;
use langchainx_testsuite::contracts::embedder::{
    assert_embedder_contract, assert_empty_documents_contract,
};

use common::FakeEmbedder;

#[tokio::test]
async fn fake_embedder_satisfies_contract() {
    let embedder = FakeEmbedder::new(128);
    assert_embedder_contract(&embedder).await;
    assert_empty_documents_contract(&embedder).await;
}

#[tokio::test]
async fn fake_embedder_different_inputs_differ() {
    let embedder = FakeEmbedder::new(64);
    let v1 = embedder.embed_query("alpha").await.unwrap();
    let v2 = embedder.embed_query("beta").await.unwrap();
    assert_ne!(
        v1, v2,
        "different inputs should produce different embeddings"
    );
}

#[test]
#[should_panic(expected = "FakeEmbedder dimensions must be nonzero")]
fn fake_embedder_rejects_zero_dimensions() {
    let _ = FakeEmbedder::new(0);
}
