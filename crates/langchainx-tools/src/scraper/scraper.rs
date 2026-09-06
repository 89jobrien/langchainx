//! HTML body text extraction that omits script contents.
use async_trait::async_trait;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use std::{net::IpAddr, sync::Arc};

use crate::{Tool, ToolError};

const MAX_HTTP_BODY_BYTES: usize = 1024 * 1024;

/// Fetches a web page and returns normalized body text with script contents removed.
pub struct WebScrapper {
    client: reqwest::Client,
    allow_private_networks: bool,
}

impl WebScrapper {
    /// Creates a stateless web scraper.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .expect("default HTTP client configuration is valid"),
            allow_private_networks: false,
        }
    }

    /// Allows requests to loopback, link-local, and private network addresses.
    pub fn with_allow_private_networks(mut self, allow: bool) -> Self {
        self.allow_private_networks = allow;
        self
    }
}

impl Default for WebScrapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebScrapper {
    fn name(&self) -> String {
        String::from("Web Scraper")
    }
    fn description(&self) -> String {
        String::from(
            "Web Scraper will scan a url and return the content of the web page.
		Input should be a working url.",
        )
    }
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let input = input
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("input must be a string".to_string()))?;
        validate_url(input, self.allow_private_networks)
            .await
            .map_err(ToolError::InvalidInput)?;
        scrape_url(&self.client, input).await
    }
}

impl From<WebScrapper> for Arc<dyn Tool> {
    fn from(ws: WebScrapper) -> Arc<dyn Tool> {
        Arc::new(ws)
    }
}

async fn scrape_url(client: &reqwest::Client, url: &str) -> Result<String, ToolError> {
    let response = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    let res = read_response_limited(response, MAX_HTTP_BODY_BYTES).await?;

    let document = Html::parse_document(&res);
    let body_selector = Selector::parse("body")
        .map_err(|error| ToolError::ExecutionFailed(format!("CSS selector error: {error:?}")))?;

    let mut text = Vec::new();
    for element in document.select(&body_selector) {
        collect_text_not_in_script(&element, &mut text);
    }

    let joined_text = text.join(" ");
    let cleaned_text = joined_text.replace(['\n', '\t'], " ");
    let re = Regex::new(r"\s+").map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
    let final_text = re.replace_all(&cleaned_text, " ");
    Ok(final_text.to_string())
}

async fn read_response_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<String, ToolError> {
    if response
        .content_length()
        .is_some_and(|length| length > limit as u64)
    {
        return Err(ToolError::ExecutionFailed(format!(
            "response body exceeds {limit} bytes"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?
    {
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(ToolError::ExecutionFailed(format!(
                "response body exceeds {limit} bytes"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|error| ToolError::ExecutionFailed(error.to_string()))
}

async fn validate_url(url: &str, allow_private_networks: bool) -> Result<(), String> {
    let parsed = reqwest::Url::parse(url).map_err(|error| format!("invalid URL: {error}"))?;
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

// qual:allow(iosp) reason: "DOM traversal with filtering"
fn collect_text_not_in_script(element: &ElementRef, text: &mut Vec<String>) {
    for node in element.children() {
        if node.value().is_element() {
            let tag_name = node.value().as_element().unwrap().name();
            if tag_name == "script" {
                continue;
            }
            collect_text_not_in_script(&ElementRef::wrap(node).unwrap(), text);
        } else if node.value().is_text() {
            text.push(node.value().as_text().unwrap().text.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_scrape_url() {
        // Request a new server from the pool
        let mut server = mockito::Server::new_async().await;

        // Create a mock on the server
        let mock = server
            .mock("GET", "/")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("<html><body>Hello World</body></html>")
            .create();

        // Instantiate your WebScrapper
        let scraper = WebScrapper::new().with_allow_private_networks(true);

        // Use the server URL for scraping
        let url = server.url();

        // Call the WebScrapper with the mocked URL
        let result = scraper.call(&url).await;

        // Assert that the result is Ok and contains "Hello World"
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content.trim(), "Hello World");

        // Verify that the mock was called as expected
        mock.assert();
    }

    #[tokio::test]
    async fn scraper_rejects_private_network_destinations_by_default() {
        let scraper = WebScrapper::new();

        let result = scraper.call("http://127.0.0.1/internal").await;

        assert!(matches!(result, Err(ToolError::InvalidInput(_))));
    }

    #[tokio::test]
    async fn scraper_rejects_oversized_response_body() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("GET", "/large")
            .with_status(200)
            .with_body(vec![b'x'; 1_048_577])
            .create_async()
            .await;
        let scraper = WebScrapper::new().with_allow_private_networks(true);

        let result = scraper.call(&format!("{}/large", server.url())).await;

        assert!(matches!(result, Err(ToolError::ExecutionFailed(_))));
        mock.assert_async().await;
    }
}
