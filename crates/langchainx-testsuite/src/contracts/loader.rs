use futures_util::StreamExt;
use langchainx_core::schemas::Document;
use langchainx_loaders::Loader;

/// Consumes a loader and collects every successful document in emission order.
pub async fn collect_documents<L: Loader>(loader: L) -> Vec<Document> {
    loader
        .load()
        .await
        .expect("Loader::load must succeed")
        .map(|result| result.expect("loader stream item must be a document"))
        .collect()
        .await
}

/// Asserts that a loader emits documents with exactly the expected page contents.
pub async fn assert_loader_contents<L: Loader>(loader: L, expected: &[&str]) {
    let documents = collect_documents(loader).await;
    let actual: Vec<&str> = documents
        .iter()
        .map(|document| document.page_content.as_str())
        .collect();
    assert_eq!(actual, expected);
}
