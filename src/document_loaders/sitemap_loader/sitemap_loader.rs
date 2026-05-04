use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use futures_util::StreamExt;

use crate::{
    document_loaders::{process_doc_stream, HtmlLoader, Loader, LoaderError},
    schemas::Document,
    text_splitter::TextSplitter,
};

pub struct SitemapLoader {
    url: String,
    client: reqwest::Client,
}

impl SitemapLoader {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_client(self, client: reqwest::Client) -> Self {
        Self { client, ..self }
    }

    async fn fetch_text(&self, url: &str) -> Result<String, LoaderError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| LoaderError::OtherError(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(LoaderError::OtherError(format!(
                "HTTP {} fetching {}",
                resp.status(),
                url
            )));
        }

        resp.text()
            .await
            .map_err(|e| LoaderError::OtherError(e.to_string()))
    }

    fn extract_locs(xml: &str, tag: &str) -> Vec<String> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut urls = Vec::new();
        let mut in_loc = false;

        loop {
            match reader.read_event() {
                Ok(Event::Start(e)) if e.name().as_ref() == b"loc" => {
                    in_loc = true;
                }
                Ok(Event::Text(e)) if in_loc => {
                    urls.push(e.unescape().unwrap_or_default().into_owned());
                    in_loc = false;
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"loc" => {
                    in_loc = false;
                }
                Ok(Event::Eof) => break,
                Err(_) => break,
                _ => {}
            }
        }

        let _ = tag; // tag used for caller context only
        urls
    }

    fn is_sitemap_index(xml: &str) -> bool {
        xml.contains("<sitemapindex")
    }

    async fn collect_docs(&self) -> Result<Vec<Document>, LoaderError> {
        let root_xml = self.fetch_text(&self.url).await?;

        let loc_urls: Vec<String> = if Self::is_sitemap_index(&root_xml) {
            // recurse one level: fetch each child sitemap and collect its locs
            let child_sitemaps = Self::extract_locs(&root_xml, "sitemapindex");
            let mut all_locs = Vec::new();
            for sitemap_url in child_sitemaps {
                let child_xml = self.fetch_text(&sitemap_url).await?;
                let locs = Self::extract_locs(&child_xml, "urlset");
                all_locs.extend(locs);
            }
            all_locs
        } else {
            Self::extract_locs(&root_xml, "urlset")
        };

        let mut docs = Vec::new();
        for loc in loc_urls {
            let url_parsed = url::Url::parse(&loc)
                .map_err(|e| LoaderError::OtherError(format!("bad url {loc}: {e}")))?;

            let html = match self.fetch_text(&loc).await {
                Ok(h) => h,
                Err(_) => continue,
            };

            let loader = HtmlLoader::from_string(html, url_parsed);
            let stream = match loader.load().await {
                Ok(s) => s,
                Err(_) => continue,
            };

            let mut page_docs: Vec<Document> = stream
                .filter_map(|r| async move { r.ok() })
                .collect()
                .await;

            // Ensure source metadata is set to the loc URL
            for doc in &mut page_docs {
                doc.metadata
                    .insert("source".to_string(), serde_json::Value::from(loc.as_str()));
            }

            docs.extend(page_docs);
        }

        Ok(docs)
    }
}

#[async_trait]
impl Loader for SitemapLoader {
    async fn load(
        self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let docs = self.collect_docs().await?;
        let stream = futures::stream::iter(docs.into_iter().map(Ok));
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
    use mockito::Server;

    use super::*;

    fn html_page(title: &str) -> String {
        format!("<html><head><title>{title}</title></head><body><p>{title} content</p></body></html>")
    }

    #[tokio::test]
    async fn test_urlset_two_locs() {
        let mut server = Server::new_async().await;

        let page1_mock = server
            .mock("GET", "/page1")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(html_page("Page One"))
            .create_async()
            .await;

        let page2_mock = server
            .mock("GET", "/page2")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(html_page("Page Two"))
            .create_async()
            .await;

        let base = server.url();
        let sitemap_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{base}/page1</loc></url>
  <url><loc>{base}/page2</loc></url>
</urlset>"#
        );

        let sitemap_mock = server
            .mock("GET", "/sitemap.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(sitemap_xml)
            .create_async()
            .await;

        let loader = SitemapLoader::new(format!("{base}/sitemap.xml"))
            .with_client(reqwest::Client::new());

        let docs: Vec<Document> = loader
            .load()
            .await
            .unwrap()
            .filter_map(|r| async move { r.ok() })
            .collect()
            .await;

        assert_eq!(docs.len(), 2, "expected 2 documents");
        let sources: Vec<&str> = docs
            .iter()
            .map(|d| d.metadata["source"].as_str().unwrap())
            .collect();
        assert!(sources.contains(&format!("{base}/page1").as_str()));
        assert!(sources.contains(&format!("{base}/page2").as_str()));

        sitemap_mock.assert_async().await;
        page1_mock.assert_async().await;
        page2_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_sitemapindex_recurses_one_level() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let page_mock = server
            .mock("GET", "/article")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(html_page("Article"))
            .create_async()
            .await;

        let child_sitemap_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{base}/article</loc></url>
</urlset>"#
        );
        let child_mock = server
            .mock("GET", "/sitemap-blog.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(child_sitemap_xml)
            .create_async()
            .await;

        let index_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>{base}/sitemap-blog.xml</loc></sitemap>
</sitemapindex>"#
        );
        let index_mock = server
            .mock("GET", "/sitemap.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(index_xml)
            .create_async()
            .await;

        let loader = SitemapLoader::new(format!("{base}/sitemap.xml"))
            .with_client(reqwest::Client::new());

        let docs: Vec<Document> = loader
            .load()
            .await
            .unwrap()
            .filter_map(|r| async move { r.ok() })
            .collect()
            .await;

        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs[0].metadata["source"].as_str().unwrap(),
            format!("{base}/article")
        );

        index_mock.assert_async().await;
        child_mock.assert_async().await;
        page_mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_404_returns_error() {
        let mut server = Server::new_async().await;

        let _mock = server
            .mock("GET", "/sitemap.xml")
            .with_status(404)
            .create_async()
            .await;

        let loader = SitemapLoader::new(format!("{}/sitemap.xml", server.url()))
            .with_client(reqwest::Client::new());

        let result = loader.load().await;
        assert!(result.is_err(), "expected Err for 404");
    }
}
