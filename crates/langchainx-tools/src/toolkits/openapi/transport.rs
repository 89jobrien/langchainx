use std::{fmt, pin::Pin};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use http::{HeaderMap, HeaderValue};
use secrecy::SecretString;
use url::Url;

use super::{AuthorizedOrigin, HttpLimits, OpenApiToolkitError};

/// Streaming post-decompression response body.
pub type HttpBody =
    Pin<Box<dyn Stream<Item = Result<Bytes, OpenApiToolkitError>> + Send + 'static>>;

/// Fully validated request supplied to an HTTP transport.
#[derive(Clone)]
pub struct HttpRequest {
    method: String,
    url: Url,
    headers: HeaderMap<SecretString>,
    body: Option<Vec<u8>>,
    authorized_origin: AuthorizedOrigin,
}

impl HttpRequest {
    pub(crate) fn new(
        method: String,
        url: Url,
        headers: HeaderMap<SecretString>,
        body: Option<Vec<u8>>,
        authorized_origin: AuthorizedOrigin,
    ) -> Self {
        Self {
            method,
            url,
            headers,
            body,
            authorized_origin,
        }
    }

    /// Returns the canonical HTTP method.
    pub fn method(&self) -> &str {
        &self.method
    }
    /// Returns the final request URL.
    pub fn url(&self) -> &Url {
        &self.url
    }
    /// Returns secret request headers without exposing them through Debug.
    pub fn headers(&self) -> &HeaderMap<SecretString> {
        &self.headers
    }
    /// Returns the optional request body.
    pub fn body(&self) -> Option<&[u8]> {
        self.body.as_deref()
    }
    /// Returns the pinned origin authorization.
    pub fn authorized_origin(&self) -> &AuthorizedOrigin {
        &self.authorized_origin
    }

    pub(crate) fn into_parts(
        self,
    ) -> (
        String,
        Url,
        HeaderMap<SecretString>,
        Option<Vec<u8>>,
        AuthorizedOrigin,
    ) {
        (
            self.method,
            self.url,
            self.headers,
            self.body,
            self.authorized_origin,
        )
    }
}

impl fmt::Debug for HttpRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("origin", self.authorized_origin.origin())
            .field("path", &self.url.path())
            .field("header_names", &self.headers.keys().collect::<Vec<_>>())
            .field("has_body", &self.body.is_some())
            .finish()
    }
}

/// Streaming HTTP response returned by a transport.
pub struct HttpResponse {
    status: u16,
    headers: HeaderMap<HeaderValue>,
    body: HttpBody,
}

impl HttpResponse {
    /// Creates a streaming response.
    pub fn new(status: u16, headers: HeaderMap<HeaderValue>, body: HttpBody) -> Self {
        Self {
            status,
            headers,
            body,
        }
    }
    /// Returns the status code.
    pub fn status(&self) -> u16 {
        self.status
    }
    /// Returns response headers.
    pub fn headers(&self) -> &HeaderMap<HeaderValue> {
        &self.headers
    }
    /// Consumes the response and returns its body stream.
    pub fn into_body(self) -> HttpBody {
        self.body
    }
}

/// Port used by generated tools to execute already-authorized HTTP requests.
///
/// Implementors must enforce `HttpLimits::header_bytes` before returning a response and must not
/// expose response headers until that check succeeds. Body stream items must contain
/// post-decompression bytes so the caller's response-body limit applies to expanded content.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// Executes one request under the supplied bounds.
    ///
    /// # Errors
    ///
    /// Returns an error when transport setup or execution fails, or when response headers exceed
    /// the configured pre-return bound.
    async fn execute(
        &self,
        request: HttpRequest,
        limits: HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn request_debug_redacts_query_headers_body_and_addresses() {
        let url = Url::parse("https://example.com/a?token=secret").unwrap();
        let origin = super::super::HttpOrigin::try_from_url(&url).unwrap();
        let authorized =
            AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))]).unwrap();
        let mut headers = HeaderMap::<SecretString>::with_capacity(1);
        headers.insert("authorization", SecretString::from("secret"));
        let request = HttpRequest::new(
            "GET".into(),
            url,
            headers,
            Some(b"secret-body".to_vec()),
            authorized,
        );
        let debug = format!("{request:?}");
        assert!(!debug.contains("token=secret"));
        assert!(!debug.contains("secret-body"));
        assert!(!debug.contains("8.8.8.8"));
        assert!(!debug.contains("\"secret\""));
    }
}
