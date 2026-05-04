use std::{
    collections::HashMap,
    io::{BufRead, Read},
    pin::Pin,
};

use async_trait::async_trait;
use futures::{stream, Stream};
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;
use serde_json::Value;

use crate::{process_doc_stream, Loader, LoaderError};

pub struct RssLoader<R> {
    input: R,
}

impl<R: Read + BufRead + Send + Sync + 'static> RssLoader<R> {
    pub fn new(input: R) -> Self {
        Self { input }
    }
}

#[async_trait]
impl<R: Read + BufRead + Send + Sync + 'static> Loader for RssLoader<R> {
    async fn load(
        self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let input = self.input;
        let channel = tokio::task::spawn_blocking(move || {
            rss::Channel::read_from(input)
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))
        })
        .await??;

        let docs: Vec<Result<Document, LoaderError>> = channel
            .items()
            .iter()
            .filter_map(|item| {
                let content = item
                    .content()
                    .filter(|s| !s.is_empty())
                    .or_else(|| item.description().filter(|s| !s.is_empty()));

                match content {
                    None => {
                        log::warn!(
                            "RssLoader: skipping item with no content or description \
                             (title={:?})",
                            item.title()
                        );
                        None
                    }
                    Some(text) => {
                        let mut metadata: HashMap<String, Value> = HashMap::new();
                        if let Some(t) = item.title() {
                            metadata.insert("title".to_string(), Value::String(t.to_string()));
                        }
                        if let Some(l) = item.link() {
                            metadata.insert("link".to_string(), Value::String(l.to_string()));
                        }
                        if let Some(d) = item.pub_date() {
                            metadata.insert(
                                "pub_date".to_string(),
                                Value::String(d.to_string()),
                            );
                        }
                        if let Some(a) = item.author() {
                            metadata.insert("author".to_string(), Value::String(a.to_string()));
                        }
                        Some(Ok(Document::new(text.to_string()).with_metadata(metadata)))
                    }
                }
            })
            .collect();

        Ok(Box::pin(stream::iter(docs)))
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
    use futures::StreamExt;
    use std::io::Cursor;

    use super::*;

    const VALID_FEED: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Item One</title>
      <link>https://example.com/1</link>
      <description>First item description</description>
      <pubDate>Mon, 01 Jan 2024 00:00:00 GMT</pubDate>
      <author>alice@example.com</author>
    </item>
    <item>
      <title>Item Two</title>
      <link>https://example.com/2</link>
      <description>Second item</description>
    </item>
  </channel>
</rss>"#;

    const FEED_WITH_EMPTY_ITEM: &str = r#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Test Feed</title>
    <item>
      <title>Has Content</title>
      <link>https://example.com/1</link>
      <description>Some content here</description>
    </item>
    <item>
      <title>Empty Item</title>
      <link>https://example.com/2</link>
    </item>
  </channel>
</rss>"#;

    const INVALID_XML: &str = r#"this is not xml at all <<<"#;

    #[tokio::test]
    async fn test_valid_feed_two_items() {
        let loader = RssLoader::new(Cursor::new(VALID_FEED.as_bytes()));
        let stream = loader.load().await.expect("load should succeed");
        let docs: Vec<_> = stream.collect().await;

        assert_eq!(docs.len(), 2, "expected 2 documents");

        let doc0 = docs[0].as_ref().expect("first doc should be Ok");
        assert_eq!(doc0.page_content, "First item description");
        assert_eq!(
            doc0.metadata.get("title").and_then(|v| v.as_str()),
            Some("Item One")
        );
        assert_eq!(
            doc0.metadata.get("link").and_then(|v| v.as_str()),
            Some("https://example.com/1")
        );
        assert_eq!(
            doc0.metadata.get("author").and_then(|v| v.as_str()),
            Some("alice@example.com")
        );
        assert!(doc0.metadata.contains_key("pub_date"));

        let doc1 = docs[1].as_ref().expect("second doc should be Ok");
        assert_eq!(doc1.page_content, "Second item");
    }

    #[tokio::test]
    async fn test_empty_item_skipped() {
        let loader = RssLoader::new(Cursor::new(FEED_WITH_EMPTY_ITEM.as_bytes()));
        let stream = loader.load().await.expect("load should succeed");
        let docs: Vec<_> = stream.collect().await;

        assert_eq!(docs.len(), 1, "empty item should be skipped");
        let doc = docs[0].as_ref().expect("doc should be Ok");
        assert_eq!(doc.page_content, "Some content here");
    }

    #[tokio::test]
    async fn test_invalid_xml_returns_error() {
        let loader = RssLoader::new(Cursor::new(INVALID_XML.as_bytes()));
        let result = loader.load().await;
        assert!(result.is_err(), "invalid XML should return LoaderError");
    }
}
