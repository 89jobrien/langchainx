use std::pin::Pin;

use async_trait::async_trait;
use futures::{stream, Stream};

use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;

use crate::{process_doc_stream, Loader, LoaderError};

/// Splits YAML frontmatter (delimited by `---`) from body content.
/// Returns `(metadata_lines, body)`.
fn parse_frontmatter(content: &str) -> (Vec<(String, String)>, String) {
    let mut lines = content.lines();
    let first = lines.next().unwrap_or("");
    if first.trim() != "---" {
        return (vec![], content.to_string());
    }
    let mut meta_lines: Vec<(String, String)> = vec![];
    let mut rest_lines: Vec<&str> = vec![];
    let mut in_front = true;
    for line in lines {
        if in_front {
            if line.trim() == "---" {
                in_front = false;
            } else if let Some((k, v)) = line.split_once(':') {
                meta_lines.push((k.trim().to_string(), v.trim().to_string()));
            } else {
                // key with no colon — key present, empty value
                meta_lines.push((line.trim().to_string(), String::new()));
            }
        } else {
            rest_lines.push(line);
        }
    }
    let body = rest_lines.join("\n");
    (meta_lines, body)
}

/// Loads markdown content, strips YAML frontmatter into metadata.
#[derive(Debug, Clone)]
pub struct MarkdownLoader {
    content: String,
}

impl MarkdownLoader {
    pub fn new<T: Into<String>>(content: T) -> Self {
        Self {
            content: content.into(),
        }
    }
}

#[async_trait]
impl Loader for MarkdownLoader {
    async fn load(
        self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let (meta_pairs, body) = parse_frontmatter(&self.content);
        let mut doc = Document::new(body);
        for (k, v) in meta_pairs {
            doc.metadata
                .insert(k, serde_json::Value::String(v));
        }
        let stream = stream::iter(vec![Ok(doc)]);
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

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;

    use super::*;

    #[tokio::test]
    async fn test_no_frontmatter_empty_metadata_full_content() {
        let md = "# Hello\n\nThis is content.";
        let loader = MarkdownLoader::new(md);
        let mut stream = loader.load().await.unwrap();
        let doc = stream.next().await.unwrap().unwrap();
        assert_eq!(doc.page_content, md);
        assert!(doc.metadata.is_empty());
    }

    #[tokio::test]
    async fn test_frontmatter_populates_metadata_and_strips_body() {
        let md = "---\ntitle: My Post\nauthor: Alice\n---\n# Body\n\nContent here.";
        let loader = MarkdownLoader::new(md);
        let mut stream = loader.load().await.unwrap();
        let doc = stream.next().await.unwrap().unwrap();
        assert_eq!(
            doc.metadata.get("title"),
            Some(&serde_json::Value::String("My Post".into()))
        );
        assert_eq!(
            doc.metadata.get("author"),
            Some(&serde_json::Value::String("Alice".into()))
        );
        assert_eq!(doc.page_content, "# Body\n\nContent here.");
    }

    #[tokio::test]
    async fn test_key_with_missing_value_has_empty_string() {
        let md = "---\ntitle: My Post\ntags\n---\nBody.";
        let loader = MarkdownLoader::new(md);
        let mut stream = loader.load().await.unwrap();
        let doc = stream.next().await.unwrap().unwrap();
        assert_eq!(
            doc.metadata.get("tags"),
            Some(&serde_json::Value::String(String::new()))
        );
    }

    #[tokio::test]
    async fn test_empty_file_empty_content_and_metadata() {
        let loader = MarkdownLoader::new("");
        let mut stream = loader.load().await.unwrap();
        let doc = stream.next().await.unwrap().unwrap();
        assert_eq!(doc.page_content, "");
        assert!(doc.metadata.is_empty());
    }
}
