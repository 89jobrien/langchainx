use langchainx_embedding::Embedder;

/// Asserts cardinality, dimensional consistency, nonzero output, and determinism.
pub async fn assert_embedder_contract(embedder: &dyn Embedder) {
    let query = embedder
        .embed_query("hello world")
        .await
        .expect("Embedder::embed_query must succeed");
    assert!(!query.is_empty(), "query embedding must not be empty");

    let documents = vec!["doc one".into(), "doc two".into(), "doc three".into()];
    let embeddings = embedder
        .embed_documents(&documents)
        .await
        .expect("Embedder::embed_documents must succeed");
    assert_eq!(embeddings.len(), documents.len());
    assert!(
        embeddings
            .iter()
            .all(|embedding| embedding.len() == query.len()),
        "document and query embedding dimensions must match"
    );
    let squared_norm: f64 = query.iter().map(|value| value * value).sum();
    assert!(squared_norm > 1e-10, "query embedding must not be zero");
    assert_eq!(
        query,
        embedder
            .embed_query("hello world")
            .await
            .expect("repeated Embedder::embed_query must succeed"),
        "identical input must produce identical embeddings"
    );
}

/// Asserts that embedding an empty document batch returns an empty vector.
pub async fn assert_empty_documents_contract(embedder: &dyn Embedder) {
    let embeddings = embedder
        .embed_documents(&[])
        .await
        .expect("empty Embedder::embed_documents must succeed");
    assert!(embeddings.is_empty());
}
