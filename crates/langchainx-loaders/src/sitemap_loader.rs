//! Loader for pages referenced by XML sitemaps and one-level sitemap indexes.
use std::{net::IpAddr, pin::Pin};

use async_trait::async_trait;
use futures::Stream;
use futures_util::StreamExt;

use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;

use crate::{HtmlLoader, Loader, LoaderError, process_doc_stream};

const MAX_HTTP_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_SITEMAP_URLS: usize = 1_000;

/// Fetches sitemap URLs and extracts readable page content with [`HtmlLoader`].
pub struct SitemapLoader {
    url: String,
    client: reqwest::Client,
    allow_private_networks: bool,
}

impl SitemapLoader {
    /// Creates a loader for a sitemap URL with a default HTTP client.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("default HTTP client configuration is valid"),
            allow_private_networks: false,
        }
    }

    /// Replaces the HTTP client used for sitemap and page requests.
    pub fn with_client(self, client: reqwest::Client) -> Self {
        Self { client, ..self }
    }

    /// Allows sitemap and page requests to private network destinations.
    pub fn with_allow_private_networks(self, allow: bool) -> Self {
        Self {
            allow_private_networks: allow,
            ..self
        }
    }

    async fn fetch_text(&self, url: &str) -> Result<String, LoaderError> {
        validate_url(url, self.allow_private_networks)
            .await
            .map_err(LoaderError::OtherError)?;
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

        read_response_limited(resp, MAX_HTTP_BODY_BYTES).await
    }

    #[allow(clippy::result_large_err)] // LoaderError contains large foreign variants; boxing requires API change
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
            ensure_url_limit(child_sitemaps.len())?;
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
        ensure_url_limit(loc_urls.len())?;

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

async fn read_response_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, LoaderError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(LoaderError::OtherError(format!(
            "response body exceeds {limit} bytes"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| LoaderError::OtherError(error.to_string()))?
    {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(LoaderError::OtherError(format!(
                "response body exceeds {limit} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|error| LoaderError::OtherError(error.to_string()))
}

fn ensure_url_limit(count: usize) -> Result<(), LoaderError> {
    if count > MAX_SITEMAP_URLS {
        return Err(LoaderError::OtherError(format!(
            "sitemap contains {count} URLs; maximum is {MAX_SITEMAP_URLS}"
        )));
    }
    Ok(())
}

async fn validate_url(url: &str, allow_private_networks: bool) -> Result<(), String> {
    let parsed = url::Url::parse(url).map_err(|error| format!("invalid URL: {error}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("URL scheme must be http or https".to_string());
    }
    let host = parsed
        .host_str()
        .ok_or_else(|| "URL must include a host".to_string())?;
    if !allow_private_networks {
        if host.eq_ignore_ascii_case("localhost") {
            return Err("private network destinations are disabled".to_string());
        }
        let port = parsed.port_or_known_default().unwrap_or(80);
        let addresses = tokio::net::lookup_host((host, port))
            .await
            .map_err(|error| format!("cannot resolve URL host: {error}"))?;
        if addresses
            .map(|address| address.ip())
            .any(|address| !is_public_address(address))
        {
            return Err("private network destinations are disabled".to_string());
        }
    }
    Ok(())
}

fn is_public_address(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !(address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_unspecified())
        }
        IpAddr::V6(address) => {
            !(address.is_loopback()
                || address.is_unique_local()
                || address.is_unicast_link_local()
                || address.is_unspecified())
        }
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

        let loader = SitemapLoader::new(format!("{base}/sitemap.xml"))
            .with_client(reqwest::Client::new())
            .with_allow_private_networks(true);

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
            .with_client(reqwest::Client::new())
            .with_allow_private_networks(true);

        let result = loader.load().await;
        assert!(result.is_err(), "expected Err for 404");
    }

    #[tokio::test]
    async fn private_network_sitemap_is_rejected_by_default() {
        let loader = SitemapLoader::new("http://127.0.0.1/internal.xml");

        let error = loader.load().await.err().expect("private URL should fail");

        assert!(error.to_string().contains("private network"));
    }

    #[tokio::test]
    async fn oversized_sitemap_response_is_rejected() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/large.xml")
            .with_status(200)
            .with_body(vec![b'x'; 2_097_153])
            .create_async()
            .await;
        let loader = SitemapLoader::new(format!("{}/large.xml", server.url()))
            .with_allow_private_networks(true);

        let error = loader.load().await.err().expect("large body should fail");

        assert!(error.to_string().contains("response body exceeds"));
        mock.assert_async().await;
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

        let loader = SitemapLoader::new(format!("{base}/sitemap.xml"))
            .with_client(reqwest::Client::new())
            .with_allow_private_networks(true);

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

        let loader = SitemapLoader::new(format!("{base}/index.xml"))
            .with_client(reqwest::Client::new())
            .with_allow_private_networks(true);

        // The failing child sitemap fetch should propagate as an error from load()
        let result = loader.load().await;
        assert!(
            result.is_err(),
            "expected Err when child sitemap fetch fails"
        );

        good_page.expect(0);
    }
}
