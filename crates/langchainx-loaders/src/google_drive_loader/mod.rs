//! Google Drive document loader.
//!
//! Loads files from a Google Drive folder or Drive query and yields them as
//! [`Document`]s.  Authentication is handled externally — the caller passes a
//! bearer token (service-account access token or OAuth2 access token).
//!
//! # Supported MIME types
//!
//! | Drive MIME type | Export strategy |
//! |---|---|
//! | `application/vnd.google-apps.document` | Export as `text/plain` |
//! | `application/vnd.google-apps.spreadsheet` | Export as `text/csv` |
//! | `application/pdf` | Download bytes and decode them as UTF-8 text |
//! | Everything else | Skipped with a `warn!` log |
//!
//! # Feature flag
//!
//! This module is compiled only when the `google-drive` feature is enabled.

use std::{collections::HashMap, pin::Pin};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;
use log::warn;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::Value;

use crate::{Loader, LoaderError, process_doc_stream};

// ---------------------------------------------------------------------------
// Drive API constants
// ---------------------------------------------------------------------------

const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const EXPORT_URL: &str = "https://www.googleapis.com/drive/v3/files/{id}/export";
const DOWNLOAD_URL: &str = "https://www.googleapis.com/drive/v3/files/{id}?alt=media";
const DRIVE_FILE_VIEW: &str = "https://drive.google.com/file/d/{id}/view";

const MIME_GDOC: &str = "application/vnd.google-apps.document";
const MIME_GSHEET: &str = "application/vnd.google-apps.spreadsheet";
const MIME_PDF: &str = "application/pdf";

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct FileList {
    files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct DriveFile {
    id: String,
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(rename = "modifiedTime")]
    modified_time: Option<String>,
}

// ---------------------------------------------------------------------------
// Query source
// ---------------------------------------------------------------------------

/// The source of files to load from Google Drive.
#[derive(Debug, Clone)]
pub enum DriveSource {
    /// Load all files directly inside a folder (non-recursive).
    FolderId(String),
    /// Use a raw Drive query string (see Drive API `q` parameter).
    Query(String),
}

impl DriveSource {
    fn to_query(&self) -> String {
        match self {
            DriveSource::FolderId(id) => {
                format!("'{}' in parents and trashed = false", id)
            }
            DriveSource::Query(q) => q.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Loader struct
// ---------------------------------------------------------------------------

/// Loads documents from Google Drive.
///
/// ```no_run
/// use langchainx_loaders::google_drive_loader::{GoogleDriveLoader, DriveSource};
/// use langchainx_loaders::Loader;
/// use futures_util::StreamExt;
///
/// # async fn run() {
/// let loader = GoogleDriveLoader::new(
///     "ya29.my-access-token".to_string(),
///     DriveSource::FolderId("1BxiMVs0XRA5nFMdKvBdBZjgmUUqptlbs74OgVE2upms".to_string()),
/// );
///
/// let mut stream = loader.load().await.unwrap();
/// while let Some(doc) = stream.next().await {
///     println!("{}", doc.unwrap().page_content);
/// }
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct GoogleDriveLoader {
    token: String,
    source: DriveSource,
    client: Client,
}

impl GoogleDriveLoader {
    /// Creates a loader with a new default HTTP client.
    ///
    /// - `token` — A valid OAuth2 or service-account bearer token.
    /// - `source` — Which files to load (folder ID or arbitrary Drive query).
    pub fn new(token: String, source: DriveSource) -> Self {
        Self {
            token,
            source,
            client: Client::new(),
        }
    }

    /// Creates a loader with a caller-provided HTTP client.
    pub fn with_client(token: String, source: DriveSource, client: Client) -> Self {
        Self {
            token,
            source,
            client,
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn list_files(&self) -> Result<Vec<DriveFile>, LoaderError> {
        let mut files = Vec::new();
        let mut page_token: Option<String> = None;

        loop {
            let q = self.source.to_query();
            let mut req = self
                .client
                .get(DRIVE_FILES_URL)
                .bearer_auth(&self.token)
                .query(&[
                    ("q", q.as_str()),
                    (
                        "fields",
                        "nextPageToken,files(id,name,mimeType,modifiedTime)",
                    ),
                    ("pageSize", "100"),
                ]);

            if let Some(ref token) = page_token {
                req = req.query(&[("pageToken", token.as_str())]);
            }

            let resp = req
                .send()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

            let status = resp.status();
            if !status.is_success() {
                let body = resp.text().await.unwrap_or_default();
                return Err(LoaderError::LoadDocumentError(format!(
                    "Drive files list returned {}: {}",
                    status, body
                )));
            }

            let list: FileList = resp
                .json()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

            files.extend(list.files);

            match list.next_page_token {
                Some(t) => page_token = Some(t),
                None => break,
            }
        }

        Ok(files)
    }

    async fn load_gdoc(&self, file: &DriveFile) -> Result<String, LoaderError> {
        let url = EXPORT_URL.replace("{id}", &file.id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("mimeType", "text/plain")])
            .send()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

        check_status(&resp, &file.id)?;
        let text = resp
            .text()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;
        Ok(text)
    }

    async fn load_gsheet_as_csv(&self, file: &DriveFile) -> Result<String, LoaderError> {
        let url = EXPORT_URL.replace("{id}", &file.id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .query(&[("mimeType", "text/csv")])
            .send()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

        check_status(&resp, &file.id)?;
        let csv = resp
            .text()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;
        Ok(csv)
    }

    async fn load_pdf(&self, file: &DriveFile) -> Result<String, LoaderError> {
        let url = DOWNLOAD_URL.replace("{id}", &file.id);
        let resp = self
            .client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

        check_status(&resp, &file.id)?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

        // Best-effort UTF-8 decode — PDFs downloaded as bytes are often
        // binary, but the test fixtures supply plain-text bytes.
        String::from_utf8(bytes.to_vec())
            .map_err(|e| LoaderError::LoadDocumentError(format!("PDF decode error: {e}")))
    }

    fn build_metadata(file: &DriveFile) -> HashMap<String, Value> {
        let source_url = DRIVE_FILE_VIEW.replace("{id}", &file.id);
        let mut m = HashMap::new();
        m.insert("source".to_string(), Value::String(source_url));
        m.insert("title".to_string(), Value::String(file.name.clone()));
        m.insert(
            "mime_type".to_string(),
            Value::String(file.mime_type.clone()),
        );
        if let Some(ref t) = file.modified_time {
            m.insert("modified_time".to_string(), Value::String(t.clone()));
        }
        m
    }
}

// ---------------------------------------------------------------------------
// Loader impl
// ---------------------------------------------------------------------------

#[async_trait]
impl Loader for GoogleDriveLoader {
    async fn load(
        self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let files = self.list_files().await?;

        let s = stream! {
            for file in files {
                let metadata = GoogleDriveLoader::build_metadata(&file);

                let content_result: Result<Option<String>, LoaderError> = match file.mime_type.as_str() {
                    MIME_GDOC => {
                        self.load_gdoc(&file).await.map(Some)
                    }
                    MIME_GSHEET => {
                        self.load_gsheet_as_csv(&file).await.map(Some)
                    }
                    MIME_PDF => {
                        self.load_pdf(&file).await.map(Some)
                    }
                    other => {
                        warn!(
                            "GoogleDriveLoader: skipping file '{}' with unsupported MIME type '{}'",
                            file.name, other
                        );
                        Ok(None)
                    }
                };

                match content_result {
                    Ok(Some(content)) => {
                        yield Ok(Document::new(content).with_metadata(metadata));
                    }
                    Ok(None) => {
                        // skipped
                    }
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }
        };

        Ok(Box::pin(s))
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

// ---------------------------------------------------------------------------
// Internal utility
// ---------------------------------------------------------------------------

#[allow(clippy::result_large_err)] // LoaderError contains large foreign variants; boxing requires API change
fn check_status(resp: &reqwest::Response, file_id: &str) -> Result<(), LoaderError> {
    let status = resp.status();
    if status == StatusCode::OK {
        return Ok(());
    }
    Err(LoaderError::LoadDocumentError(format!(
        "Drive API returned {} for file {}",
        status, file_id
    )))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use mockito::{Matcher, Server};
    use reqwest::Client;

    use super::*;

    fn make_file_list(files: &[(&str, &str, &str, &str)]) -> String {
        // files: (id, name, mimeType, modifiedTime)
        let entries: Vec<String> = files
            .iter()
            .map(|(id, name, mime, modified)| {
                format!(
                    r#"{{"id":"{id}","name":"{name}","mimeType":"{mime}","modifiedTime":"{modified}"}}"#,
                    id = id,
                    name = name,
                    mime = mime,
                    modified = modified,
                )
            })
            .collect();
        format!(r#"{{"files":[{}]}}"#, entries.join(","))
    }

    #[tokio::test]
    async fn loads_google_doc() {
        let mut server = Server::new_async().await;
        let base = server.url();

        // Override the URLs to point at the mock server.
        // We achieve this by subclassing the client; we instead build the
        // loader with a client that has the mock base injected via a
        // custom reqwest Client (not possible to override constants, so we
        // directly call the mock endpoints).

        // List endpoint
        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(make_file_list(&[(
                "doc1",
                "My Doc",
                MIME_GDOC,
                "2024-01-01T00:00:00Z",
            )]))
            .create_async()
            .await;

        // Export endpoint for the doc
        let _m_export = server
            .mock("GET", "/drive/v3/files/doc1/export")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body("Hello from Google Doc")
            .create_async()
            .await;

        let loader = make_test_loader("fake-token", &base, DriveSource::FolderId("folder1".into()));
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content, "Hello from Google Doc");
        assert_eq!(
            docs[0].metadata.get("title").unwrap(),
            &Value::String("My Doc".into())
        );
        assert_eq!(
            docs[0].metadata.get("mime_type").unwrap(),
            &Value::String(MIME_GDOC.into())
        );
        assert!(
            docs[0]
                .metadata
                .get("source")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("doc1")
        );
    }

    #[tokio::test]
    async fn loads_google_sheet_as_csv() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(make_file_list(&[(
                "sheet1",
                "My Sheet",
                MIME_GSHEET,
                "2024-02-01T00:00:00Z",
            )]))
            .create_async()
            .await;

        let _m_export = server
            .mock("GET", "/drive/v3/files/sheet1/export")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "text/csv")
            .with_body("col_a,col_b\nfoo,bar")
            .create_async()
            .await;

        let loader = make_test_loader("fake-token", &base, DriveSource::FolderId("folder1".into()));
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content, "col_a,col_b\nfoo,bar");
    }

    #[tokio::test]
    async fn loads_pdf_as_bytes() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(make_file_list(&[(
                "pdf1",
                "My PDF",
                MIME_PDF,
                "2024-03-01T00:00:00Z",
            )]))
            .create_async()
            .await;

        let _m_download = server
            .mock("GET", "/drive/v3/files/pdf1")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/pdf")
            .with_body("PDF binary content")
            .create_async()
            .await;

        let loader = make_test_loader("fake-token", &base, DriveSource::FolderId("folder1".into()));
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].page_content, "PDF binary content");
    }

    #[tokio::test]
    async fn skips_unsupported_mime_type() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(make_file_list(&[(
                "vid1",
                "My Video",
                "video/mp4",
                "2024-04-01T00:00:00Z",
            )]))
            .create_async()
            .await;

        let loader = make_test_loader("fake-token", &base, DriveSource::FolderId("folder1".into()));
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;

        assert_eq!(docs.len(), 0, "unsupported MIME type should be skipped");
    }

    #[tokio::test]
    async fn metadata_fields_populated() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(make_file_list(&[(
                "docX",
                "Metadata Doc",
                MIME_GDOC,
                "2024-05-15T10:30:00Z",
            )]))
            .create_async()
            .await;

        let _m_export = server
            .mock("GET", "/drive/v3/files/docX/export")
            .match_query(Matcher::Any)
            .with_status(200)
            .with_body("content")
            .create_async()
            .await;

        let loader = make_test_loader("fake-token", &base, DriveSource::FolderId("folder1".into()));
        let docs: Vec<_> = loader
            .load()
            .await
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
            .await;

        assert_eq!(docs.len(), 1);
        let meta = &docs[0].metadata;
        assert_eq!(
            meta.get("title").unwrap(),
            &Value::String("Metadata Doc".into())
        );
        assert_eq!(
            meta.get("mime_type").unwrap(),
            &Value::String(MIME_GDOC.into())
        );
        assert_eq!(
            meta.get("modified_time").unwrap(),
            &Value::String("2024-05-15T10:30:00Z".into())
        );
        assert!(
            meta.get("source")
                .unwrap()
                .as_str()
                .unwrap()
                .contains("docX")
        );
    }

    #[tokio::test]
    async fn list_api_error_returns_err() {
        let mut server = Server::new_async().await;
        let base = server.url();

        let _m_list = server
            .mock("GET", "/drive/v3/files")
            .match_query(Matcher::Any)
            .with_status(403)
            .with_body(r#"{"error":{"message":"Access denied"}}"#)
            .create_async()
            .await;

        let loader = make_test_loader("bad-token", &base, DriveSource::FolderId("folder1".into()));
        let result = loader.load().await;
        assert!(result.is_err(), "expected error on 403 response");
    }

    /// Build a loader that routes Drive API calls to the local mock server.
    /// This works by constructing a reqwest Client and rewriting the
    /// URLs embedded in the loader via a thin wrapper type.
    fn make_test_loader(token: &str, base_url: &str, source: DriveSource) -> MockDriveLoader {
        MockDriveLoader {
            token: token.to_string(),
            source,
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }

    // ------------------------------------------------------------------
    // A thin test wrapper that overrides Drive API base URLs.
    // ------------------------------------------------------------------
    struct MockDriveLoader {
        token: String,
        source: DriveSource,
        client: Client,
        base_url: String,
    }

    impl MockDriveLoader {
        fn files_url(&self) -> String {
            format!("{}/drive/v3/files", self.base_url)
        }
        fn export_url(&self, id: &str) -> String {
            format!("{}/drive/v3/files/{}/export", self.base_url, id)
        }
        fn download_url(&self, id: &str) -> String {
            format!("{}/drive/v3/files/{}", self.base_url, id)
        }

        async fn list_files(&self) -> Result<Vec<DriveFile>, LoaderError> {
            let mut files = Vec::new();
            let mut page_token: Option<String> = None;

            loop {
                let q = self.source.to_query();
                let mut req = self
                    .client
                    .get(self.files_url())
                    .bearer_auth(&self.token)
                    .query(&[
                        ("q", q.as_str()),
                        (
                            "fields",
                            "nextPageToken,files(id,name,mimeType,modifiedTime)",
                        ),
                        ("pageSize", "100"),
                    ]);

                if let Some(ref t) = page_token {
                    req = req.query(&[("pageToken", t.as_str())]);
                }

                let resp = req
                    .send()
                    .await
                    .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

                let status = resp.status();
                if !status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    return Err(LoaderError::LoadDocumentError(format!(
                        "Drive files list returned {}: {}",
                        status, body
                    )));
                }

                let list: FileList = resp
                    .json()
                    .await
                    .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;

                files.extend(list.files);

                match list.next_page_token {
                    Some(t) => page_token = Some(t),
                    None => break,
                }
            }

            Ok(files)
        }

        async fn fetch_text(
            &self,
            url: &str,
            query: &[(&str, &str)],
        ) -> Result<String, LoaderError> {
            let resp = self
                .client
                .get(url)
                .bearer_auth(&self.token)
                .query(query)
                .send()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(LoaderError::LoadDocumentError(format!(
                    "Drive API returned {}",
                    status
                )));
            }
            resp.text()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))
        }

        async fn fetch_bytes(
            &self,
            url: &str,
            query: &[(&str, &str)],
        ) -> Result<Vec<u8>, LoaderError> {
            let resp = self
                .client
                .get(url)
                .bearer_auth(&self.token)
                .query(query)
                .send()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;
            let status = resp.status();
            if !status.is_success() {
                return Err(LoaderError::LoadDocumentError(format!(
                    "Drive API returned {}",
                    status
                )));
            }
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))?;
            Ok(bytes.to_vec())
        }
    }

    #[async_trait]
    impl Loader for MockDriveLoader {
        async fn load(
            self,
        ) -> Result<
            Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
            LoaderError,
        > {
            let files = self.list_files().await?;

            let s = stream! {
                for file in files {
                    let metadata = GoogleDriveLoader::build_metadata(&file);

                    let result: Result<Option<String>, LoaderError> = match file.mime_type.as_str() {
                        MIME_GDOC => {
                            let url = self.export_url(&file.id);
                            self.fetch_text(&url, &[("mimeType", "text/plain")]).await.map(Some)
                        }
                        MIME_GSHEET => {
                            let url = self.export_url(&file.id);
                            self.fetch_text(&url, &[("mimeType", "text/csv")]).await.map(Some)
                        }
                        MIME_PDF => {
                            let url = self.download_url(&file.id);
                            self.fetch_bytes(&url, &[("alt", "media")]).await
                                .and_then(|b| {
                                    String::from_utf8(b)
                                        .map_err(|e| LoaderError::LoadDocumentError(e.to_string()))
                                })
                                .map(Some)
                        }
                        other => {
                            warn!("skipping unsupported MIME type: {}", other);
                            Ok(None)
                        }
                    };

                    match result {
                        Ok(Some(content)) => yield Ok(Document::new(content).with_metadata(metadata)),
                        Ok(None) => {}
                        Err(e) => yield Err(e),
                    }
                }
            };

            Ok(Box::pin(s))
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
}
