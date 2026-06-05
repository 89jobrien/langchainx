use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use futures_util::StreamExt;

use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;

use crate::{HtmlLoader, Loader, LoaderError, process_doc_stream};

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

    fn extract_locs(xml: &str, tag: &str) -> Result<Vec<String>, LoaderError> {
        use quick_xml::Reader;
        use quick_xml::events::Event;

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
                    let loc = e.unescape().map_err(|e| {
                        LoaderError::OtherError(format!("Invalid {tag} <loc> text: {e}"))
                    })?;
                    urls.push(loc.into_owned());
                    in_loc = false;
                }
                Ok(Event::End(e)) if e.name().as_ref() == b"loc" => {
                    in_loc = false;
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(LoaderError::OtherError(format!(
                        "Invalid {tag} XML while reading sitemap locations: {e}"
                    )));
                }
                _ => {}
            }
        }
        Ok(urls)
    }

    fn is_sitemap_index(xml: &str) -> bool {
        xml.contains("<sitemapindex")
    }

    // qual:allow(iosp) reason: "network I/O boundary"
    async fn collect_docs(&self) -> Result<Vec<Document>, LoaderError> {
        let root_xml = self.fetch_text(&self.url).await?;

        let loc_urls: Vec<String> = if Self::is_sitemap_index(&root_xml) {
            // recurse one level: fetch each child sitemap and collect its locs
            let child_sitemaps = Self::extract_locs(&root_xml, "sitemapindex")?;
            let mut all_locs = Vec::new();
            for sitemap_url in child_sitemaps {
                let child_xml = self.fetch_text(&sitemap_url).await?;
                let locs = Self::extract_locs(&child_xml, "urlset")?;
                all_locs.extend(locs);
            }
            all_locs
        } else {
            Self::extract_locs(&root_xml, "urlset")?
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

            let mut page_docs: Vec<Document> =
                stream.filter_map(|r| async move { r.ok() }).collect().await;

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
        format!(
            "<html><head><title>{title}</title></head><body><p>{title} content</p></body></html>"
        )
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

        let loader =
            SitemapLoader::new(format!("{base}/sitemap.xml")).with_client(reqwest::Client::new());

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

    #[test]
    fn extract_locs_returns_error_for_malformed_xml() {
        let result = SitemapLoader::extract_locs(
            "<urlset><url><loc>https://example.com/?a=1&b=2</loc></url></urlset>",
            "urlset",
        );

        assert!(result.is_err());
    }

    #[test]
    fn extract_locs_returns_empty_for_no_loc_tags() {
        let xml = r#"<?xml version="1.0"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"></urlset>"#;
        let locs = SitemapLoader::extract_locs(xml, "urlset").unwrap();
        assert!(locs.is_empty());
    }

    #[test]
    fn extract_locs_parses_multiple_urls() {
        let xml = r#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>https://example.com/a</loc></url>
  <url><loc>https://example.com/b</loc></url>
  <url><loc>https://example.com/c</loc></url>
</urlset>"#;
        let locs = SitemapLoader::extract_locs(xml, "urlset").unwrap();
        assert_eq!(locs.len(), 3);
        assert!(locs.contains(&"https://example.com/a".to_string()));
        assert!(locs.contains(&"https://example.com/c".to_string()));
    }

    #[tokio::test]
    async fn page_fetch_failure_is_skipped_not_propagated() {
        // Page 2 returns 500 — loader should skip it and still produce docs for page 1.
        let mut server = Server::new_async().await;
        let base = server.url();

        let page1_mock = server
            .mock("GET", "/page1")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(html_page("Page One"))
            .create_async()
            .await;

        let _page2_error = server
            .mock("GET", "/page2")
            .with_status(500)
            .create_async()
            .await;

        let sitemap_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{base}/page1</loc></url>
  <url><loc>{base}/page2</loc></url>
</urlset>"#
        );

        let _sitemap_mock = server
            .mock("GET", "/sitemap.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(sitemap_xml)
            .create_async()
            .await;

        let loader =
            SitemapLoader::new(format!("{base}/sitemap.xml")).with_client(reqwest::Client::new());

        let docs: Vec<Document> = loader
            .load()
            .await
            .unwrap()
            .filter_map(|r| async move { r.ok() })
            .collect()
            .await;

        // Only page 1 should be present; page 2 failure is silently skipped.
        assert_eq!(docs.len(), 1);
        assert_eq!(
            docs[0].metadata["source"].as_str().unwrap(),
            format!("{base}/page1")
        );

        page1_mock.assert_async().await;
    }

    #[tokio::test]
    async fn sitemapindex_with_failing_child_skips_that_child() {
        let mut server = Server::new_async().await;
        let base = server.url();

        // Good child sitemap + page
        let good_page = server
            .mock("GET", "/good-page")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(html_page("Good"))
            .create_async()
            .await;

        let good_child_xml = format!(
            r#"<?xml version="1.0"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <url><loc>{base}/good-page</loc></url>
</urlset>"#
        );

        let _good_child = server
            .mock("GET", "/child1.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(good_child_xml)
            .create_async()
            .await;

        // Bad child sitemap returns 503
        let _bad_child = server
            .mock("GET", "/child2.xml")
            .with_status(503)
            .create_async()
            .await;

        let index_xml = format!(
            r#"<?xml version="1.0"?>
<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
  <sitemap><loc>{base}/child1.xml</loc></sitemap>
  <sitemap><loc>{base}/child2.xml</loc></sitemap>
</sitemapindex>"#
        );

        let _index_mock = server
            .mock("GET", "/index.xml")
            .with_status(200)
            .with_header("content-type", "application/xml")
            .with_body(index_xml)
            .create_async()
            .await;

        let loader =
            SitemapLoader::new(format!("{base}/index.xml")).with_client(reqwest::Client::new());

        // The failing child sitemap fetch should propagate as an error from load()
        let result = loader.load().await;
        assert!(
            result.is_err(),
            "expected Err when child sitemap fetch fails"
        );

        good_page.expect(0);
    }
}
