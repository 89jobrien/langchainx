//! Loaders for JSON values and newline-delimited JSON records.
use crate::{Loader, LoaderError, process_doc_stream};
use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Cursor, Read};
use std::path::Path;
use std::pin::Pin;

// ──────────────────────────────── JsonLoader ────────────────────────────────

#[derive(Debug)]
/// Loads a JSON value or array into documents.
pub struct JsonLoader<R> {
    reader: R,
    content_key: Option<String>,
}

impl<R: Read> JsonLoader<R> {
    /// Creates a loader from a JSON reader.
    pub fn new(input: R) -> Self {
        Self {
            reader: input,
            content_key: None,
        }
    }

    /// Uses an object field as page content and stores remaining fields as metadata.
    pub fn with_content_key(mut self, key: impl Into<String>) -> Self {
        self.content_key = Some(key.into());
        self
    }
}

impl JsonLoader<Cursor<Vec<u8>>> {
    /// Creates a loader from JSON text.
    pub fn from_string(input: impl Into<String>) -> Self {
        let bytes = input.into().into_bytes();
        Self::new(Cursor::new(bytes))
    }
}

impl JsonLoader<BufReader<File>> {
    /// Opens a JSON file and creates a loader for it.
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError> {
        let file = File::open(path)?;
        Ok(Self::new(BufReader::new(file)))
    }
}

#[async_trait]
impl<R: Read + Send + Sync + 'static> Loader for JsonLoader<R> {
    // qual:allow(iosp) reason: "loader I/O boundary"
    async fn load(
        mut self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let mut buf = String::new();
        self.reader.read_to_string(&mut buf)?;

        let parsed: Value =
            serde_json::from_str(&buf).map_err(|e| LoaderError::OtherError(e.to_string()))?;

        let items: Vec<Value> = match parsed {
            Value::Array(arr) => arr,
            other => vec![other],
        };

        let content_key = self.content_key.clone();

        let stream = stream! {
            for item in items {
                yield doc_from_value(item, content_key.as_deref());
            }
        };

        Ok(Box::pin(stream))
    }

    async fn load_and_split<TS: TextSplitter + 'static>(
        self,
        splitter: TS,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let doc_stream = self.load().await?;
        let stream = process_doc_stream(doc_stream, splitter).await;
        Ok(Box::pin(stream))
    }
}

// ──────────────────────────────── JsonlLoader ───────────────────────────────

#[derive(Debug)]
/// Loads one JSON value per non-empty input line.
pub struct JsonlLoader<R> {
    reader: R,
    content_key: Option<String>,
}

impl<R: BufRead> JsonlLoader<R> {
    /// Creates a loader from a buffered JSON Lines reader.
    pub fn new(input: R) -> Self {
        Self {
            reader: input,
            content_key: None,
        }
    }

    /// Uses an object field as page content and stores remaining fields as metadata.
    pub fn with_content_key(mut self, key: impl Into<String>) -> Self {
        self.content_key = Some(key.into());
        self
    }
}

impl JsonlLoader<BufReader<Cursor<Vec<u8>>>> {
    /// Creates a loader from JSON Lines text.
    pub fn from_string(input: impl Into<String>) -> Self {
        let bytes = input.into().into_bytes();
        Self::new(BufReader::new(Cursor::new(bytes)))
    }
}

impl JsonlLoader<BufReader<File>> {
    /// Opens a JSON Lines file and creates a loader for it.
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError> {
        let file = File::open(path)?;
        Ok(Self::new(BufReader::new(file)))
    }
}

#[async_trait]
impl<R: BufRead + Send + Sync + 'static> Loader for JsonlLoader<R> {
    async fn load(
        mut self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let mut lines: Vec<String> = Vec::new();
        for line in self.reader.lines() {
            lines.push(line.map_err(|e| LoaderError::OtherError(e.to_string()))?);
        }

        let content_key = self.content_key.clone();

        let stream = stream! {
            for line in lines {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }
                let value: Value = match serde_json::from_str(&trimmed) {
                    Ok(v) => v,
                    Err(e) => {
                        yield Err(LoaderError::OtherError(e.to_string()));
                        continue;
                    }
                };
                yield doc_from_value(value, content_key.as_deref());
            }
        };

        Ok(Box::pin(stream))
    }

    async fn load_and_split<TS: TextSplitter + 'static>(
        self,
        splitter: TS,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let doc_stream = self.load().await?;
        let stream = process_doc_stream(doc_stream, splitter).await;
        Ok(Box::pin(stream))
    }
}

// ──────────────────────────────── helpers ───────────────────────────────────

#[allow(clippy::result_large_err)] // LoaderError contains large foreign variants; boxing requires API change
fn doc_from_value(value: Value, content_key: Option<&str>) -> Result<Document, LoaderError> {
    match content_key {
        None => {
            let page_content = serde_json::to_string(&value)
                .map_err(|e| LoaderError::OtherError(e.to_string()))?;
            Ok(Document::new(page_content))
        }
        Some(key) => {
            let mut obj = match value {
                Value::Object(m) => m,
                other => {
                    return Err(LoaderError::OtherError(format!(
                        "expected JSON object, got {other}"
                    )));
                }
            };
            let content_val = obj.remove(key).unwrap_or(Value::Null);
            let page_content = match content_val {
                Value::String(s) => s,
                other => serde_json::to_string(&other)
                    .map_err(|e| LoaderError::OtherError(e.to_string()))?,
            };
            let metadata: HashMap<String, Value> = obj.into_iter().collect();
            let mut doc = Document::new(page_content);
            doc.metadata = metadata;
            Ok(doc)
        }
    }
}

// ──────────────────────────────── tests ─────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt;

    #[tokio::test]
    async fn test_json_array_no_content_key() {
        let input = r#"[{"name":"Alice","age":30},{"name":"Bob","age":25}]"#;
        let loader = JsonLoader::from_string(input);
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 2);
        let v0: Value = serde_json::from_str(&docs[0].page_content).unwrap();
        assert_eq!(v0["name"], "Alice");
        assert!(docs[0].metadata.is_empty());
    }

    #[tokio::test]
    async fn test_json_array_with_content_key() {
        let input =
            r#"[{"text":"hello world","source":"a.txt"},{"text":"foo bar","source":"b.txt"}]"#;
        let loader = JsonLoader::from_string(input).with_content_key("text");
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].page_content, "hello world");
        assert_eq!(docs[0].metadata["source"], Value::String("a.txt".into()));
        assert_eq!(docs[1].page_content, "foo bar");
    }

    #[tokio::test]
    async fn test_jsonl_multiple_lines() {
        let input = "{\"text\":\"line one\"}\n{\"text\":\"line two\"}\n{\"text\":\"line three\"}\n";
        let loader = JsonlLoader::from_string(input).with_content_key("text");
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 3);
        assert_eq!(docs[0].page_content, "line one");
        assert_eq!(docs[2].page_content, "line three");
    }

    #[tokio::test]
    async fn test_jsonl_invalid_line_yields_error() {
        let input = "{\"text\":\"good\"}\nnot valid json\n{\"text\":\"also good\"}\n";
        let loader = JsonlLoader::from_string(input).with_content_key("text");
        let results: Vec<_> = loader.load().await.unwrap().collect().await;
        assert_eq!(results.len(), 3);
        assert!(results[0].is_ok());
        assert!(results[1].is_err());
        assert!(results[2].is_ok());
    }

    #[tokio::test]
    async fn json_loader_malformed_returns_error() {
        let loader = JsonLoader::from_string("{ not valid json }");
        let result = loader.load().await;
        assert!(result.is_err(), "expected Err for malformed JSON");
    }

    #[tokio::test]
    async fn json_loader_empty_string_returns_error() {
        let loader = JsonLoader::from_string("");
        let result = loader.load().await;
        assert!(result.is_err(), "expected Err for empty input");
    }

    #[tokio::test]
    async fn json_loader_single_object_no_key_wraps_as_single_doc() {
        let input = r#"{"hello":"world"}"#;
        let loader = JsonLoader::from_string(input);
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 1);
        let v: serde_json::Value = serde_json::from_str(&docs[0].page_content).unwrap();
        assert_eq!(v["hello"], "world");
    }

    #[tokio::test]
    async fn json_loader_content_key_on_non_object_yields_error() {
        // Array element is not an object — content_key should fail
        let input = r#"["just a string"]"#;
        let loader = JsonLoader::from_string(input).with_content_key("text");
        let results: Vec<_> = loader.load().await.unwrap().collect().await;
        assert_eq!(results.len(), 1);
        assert!(results[0].is_err());
    }

    #[tokio::test]
    async fn jsonl_empty_input_yields_no_docs() {
        let loader = JsonlLoader::from_string("");
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 0);
    }

    #[tokio::test]
    async fn jsonl_blank_lines_are_skipped() {
        let input = "{\"x\":1}\n\n\n{\"x\":2}\n";
        let loader = JsonlLoader::from_string(input);
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;
        assert_eq!(docs.len(), 2);
    }
}
