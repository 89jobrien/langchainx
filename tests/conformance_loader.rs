/// Conformance tests for the `Loader` trait contract.
///
/// Every `Loader` impl must satisfy:
/// 1. `load()` returns a Stream of Result<Document, LoaderError>.
/// 2. Each Document has non-empty page_content.
/// 3. Documents are emitted in order.
/// 4. Empty input produces a single document with empty content (not an error).
use langchainx::document_loaders::TextLoader;
use langchainx_testsuite::contracts::loader::{assert_loader_contents, collect_documents};

#[tokio::test]
async fn text_loader_produces_single_document() {
    let content = "Hello, this is a test document.";
    let loader = TextLoader::new(content);
    assert_loader_contents(loader, &[content]).await;
}

#[tokio::test]
async fn text_loader_empty_input_produces_document() {
    let loader = TextLoader::new("");
    assert_loader_contents(loader, &[""]).await;
}

#[tokio::test]
async fn text_loader_preserves_content() {
    let content = "Line 1\nLine 2\nLine 3";
    let loader = TextLoader::new(content);
    assert_loader_contents(loader, &[content]).await;
}

#[tokio::test]
async fn csv_loader_produces_one_document_per_row() {
    let csv = "name,age\nAlice,30\nBob,25\n";
    let columns = vec!["name".to_string(), "age".to_string()];
    let loader = langchainx::document_loaders::CsvLoader::new(csv.as_bytes(), columns);
    let docs = collect_documents(loader).await;

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
