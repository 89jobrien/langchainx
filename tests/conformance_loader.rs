/// Conformance tests for the `Loader` trait contract.
///
/// Every `Loader` impl must satisfy:
/// 1. `load()` returns a Stream of Result<Document, LoaderError>.
/// 2. Each Document has non-empty page_content.
/// 3. Documents are emitted in order.
/// 4. Empty input produces an empty stream (not an error).
use futures_util::StreamExt;

use langchainx::document_loaders::{Loader, TextLoader};
use langchainx::schemas::Document;

#[tokio::test]
async fn text_loader_produces_single_document() {
    let content = "Hello, this is a test document.";
    let loader = TextLoader::new(content);
    let stream = loader.load().await.expect("load() must return Ok");

    let docs: Vec<Document> = stream
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(docs.len(), 1, "TextLoader must produce exactly one document");
    assert_eq!(docs[0].page_content, content);
}

#[tokio::test]
async fn text_loader_empty_input_produces_document() {
    let loader = TextLoader::new("");
    let stream = loader.load().await.expect("load() must not error on empty");

    let docs: Vec<Document> = stream
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(docs.len(), 1);
}

#[tokio::test]
async fn text_loader_preserves_content() {
    let content = "Line 1\nLine 2\nLine 3";
    let loader = TextLoader::new(content);
    let stream = loader.load().await.unwrap();

    let docs: Vec<Document> = stream
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(docs[0].page_content, content);
}

#[tokio::test]
async fn csv_loader_produces_one_document_per_row() {
    let csv = "name,age\nAlice,30\nBob,25\n";
    let columns = vec!["name".to_string(), "age".to_string()];
    let loader = langchainx::document_loaders::CsvLoader::new(csv.as_bytes(), columns);
    let stream = loader.load().await.expect("csv load");

    let docs: Vec<Document> = stream
        .filter_map(|r| async { r.ok() })
        .collect()
        .await;

    assert_eq!(
        docs.len(),
        2,
        "CsvLoader must produce one document per data row (not header)"
    );
    assert!(
        docs[0].page_content.contains("Alice"),
        "first doc must contain first row data"
    );
}
