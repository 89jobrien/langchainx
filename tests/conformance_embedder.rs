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

use common::FakeEmbedder;

async fn assert_embedder_contract(embedder: &dyn Embedder) {
    // 1. embed_query returns non-empty vector
    let vec1 = embedder.embed_query("hello world").await.unwrap();
    assert!(!vec1.is_empty(), "embed_query must return non-empty vector");

    // 2. embed_documents returns one vector per doc
    let docs = vec!["doc one".into(), "doc two".into(), "doc three".into()];
    let vecs = embedder.embed_documents(&docs).await.unwrap();
    assert_eq!(
        vecs.len(),
        3,
        "embed_documents must return one vector per document"
    );

    // 3. All vectors same dimensionality
    let dim = vec1.len();
    for (i, v) in vecs.iter().enumerate() {
        assert_eq!(
            v.len(),
            dim,
            "vector {i} has dimension {} but expected {dim}",
            v.len()
        );
    }

    // 4. Not all zeros
    let norm: f64 = vec1.iter().map(|x| x * x).sum();
    assert!(
        norm > 1e-10,
        "embedding vector must not be all zeros (norm={norm})"
    );

    // 5. Deterministic
    let vec1b = embedder.embed_query("hello world").await.unwrap();
    assert_eq!(vec1, vec1b, "same input must produce same embedding");
}

#[tokio::test]
async fn fake_embedder_satisfies_contract() {
    let embedder = FakeEmbedder::new(128);
    assert_embedder_contract(&embedder).await;
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
