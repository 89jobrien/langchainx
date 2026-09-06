use langchainx_core::schemas::Document;
use langchainx_vectorstore::VectorStore;
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Verifies insertion IDs, result limits, zero-limit behavior, and document round trips.
pub async fn assert_vectorstore_contract<V>(store: &V, options: &V::Options)
where
    V: VectorStore,
{
    let docs = vec![
        Document::new("first").with_metadata(HashMap::from([("rank".to_string(), json!(1))])),
        Document::new("second").with_metadata(HashMap::from([("rank".to_string(), json!(2))])),
    ];
    let ids = store
        .add_documents(&docs, options)
        .await
        .expect("VectorStore::add_documents must accept valid documents");
    assert_eq!(ids.len(), docs.len(), "one ID is required per document");
    assert!(ids.iter().all(|id| !id.is_empty()), "IDs must be non-empty");
    assert_eq!(
        ids.iter().collect::<HashSet<_>>().len(),
        ids.len(),
        "document IDs must be unique"
    );

    let limited = store
        .similarity_search("query", 1, options)
        .await
        .expect("VectorStore::similarity_search must accept a positive limit");
    assert!(
        limited.len() <= 1,
        "search returned more than the requested limit"
    );
    assert!(!limited.is_empty(), "search returned no inserted documents");
    let returned = &limited[0];
    let inserted = docs
        .iter()
        .find(|document| document.page_content == returned.page_content)
        .expect("search returned a document that was not inserted");
    assert_eq!(
        returned.metadata, inserted.metadata,
        "document metadata did not round trip"
    );

    let empty = store
        .similarity_search("query", 0, options)
        .await
        .expect("VectorStore::similarity_search must accept a zero limit");
    assert!(empty.is_empty(), "a zero limit must return no documents");
}
