use crate::{process_doc_stream, Loader, LoaderError};
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
pub struct JsonLoader<R> {
    reader: R,
    content_key: Option<String>,
}

impl<R: Read> JsonLoader<R> {
    pub fn new(input: R) -> Self {
        Self {
            reader: input,
            content_key: None,
        }
    }

    pub fn with_content_key(mut self, key: impl Into<String>) -> Self {
        self.content_key = Some(key.into());
        self
    }
}

impl JsonLoader<Cursor<Vec<u8>>> {
    pub fn from_string(input: impl Into<String>) -> Self {
        let bytes = input.into().into_bytes();
        Self::new(Cursor::new(bytes))
    }
}

impl JsonLoader<BufReader<File>> {
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError> {
        let file = File::open(path)?;
        Ok(Self::new(BufReader::new(file)))
    }
}

#[async_trait]
impl<R: Read + Send + Sync + 'static> Loader for JsonLoader<R> {
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
pub struct JsonlLoader<R> {
    reader: R,
    content_key: Option<String>,
}

impl<R: BufRead> JsonlLoader<R> {
    pub fn new(input: R) -> Self {
        Self {
            reader: input,
            content_key: None,
        }
    }

    pub fn with_content_key(mut self, key: impl Into<String>) -> Self {
        self.content_key = Some(key.into());
        self
    }
}

impl JsonlLoader<BufReader<Cursor<Vec<u8>>>> {
    pub fn from_string(input: impl Into<String>) -> Self {
        let bytes = input.into().into_bytes();
        Self::new(BufReader::new(Cursor::new(bytes)))
    }
}

impl JsonlLoader<BufReader<File>> {
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
                    return Err(LoaderError::OtherError(
                        format!("expected JSON object, got {other}"),
                    ));
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
        let input = r#"[{"text":"hello world","source":"a.txt"},{"text":"foo bar","source":"b.txt"}]"#;
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
}
