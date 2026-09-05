use std::{
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use http::{HeaderMap, HeaderValue};
use secrecy::ExposeSecret;

use super::{HttpLimits, HttpRequest, HttpResponse, HttpTransport, OpenApiToolkitError};

const MAX_RESPONSE_HEADERS: usize = 100;

/// Redirect-disabled HTTP transport that connects only to policy-pinned addresses.
///
/// HTTP/2 advertises the configured byte-perfect header-list bound before receiving headers.
/// Reqwest does not expose Hyper's HTTP/1 header-buffer byte setting, so HTTP/1 retains Hyper's
/// pre-allocation limit of 100 fields and this adapter performs an additional byte-perfect check
/// immediately after header parsing and before exposing the response.
#[derive(Clone, Copy, Debug, Default)]
pub struct ReqwestTransport;

impl ReqwestTransport {
    /// Creates a pinned-address reqwest transport.
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl HttpTransport for ReqwestTransport {
    async fn execute(
        &self,
        request: HttpRequest,
        limits: HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        let (method, url, headers, body, authorized_origin) = request.into_parts();
        let origin = authorized_origin.origin();
        let sockets: Vec<_> = authorized_origin
            .addresses()
            .iter()
            .map(|address| SocketAddr::new(*address, origin.port()))
            .collect();
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(limits.timeout)
            .http2_max_header_list_size(u32::try_from(limits.header_bytes).unwrap_or(u32::MAX))
            .resolve_to_addrs(origin.host(), &sockets)
            .build()
            .map_err(|_| OpenApiToolkitError::Transport("HTTP client setup failed".into()))?;
        let method = reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|_| OpenApiToolkitError::Transport("invalid HTTP method".into()))?;
        let mut builder = client.request(method, url);
        for (name, value) in &headers {
            let value = HeaderValue::from_str(value.expose_secret())
                .map_err(|_| OpenApiToolkitError::Transport("invalid request header".into()))?;
            builder = builder.header(name, value);
        }
        if let Some(body) = body {
            builder = builder.body(body);
        }
        let response = builder
            .send()
            .await
            .map_err(|_| OpenApiToolkitError::Transport("HTTP request failed".into()))?;
        let headers: HeaderMap<HeaderValue> = response.headers().clone();
        if headers.len() > MAX_RESPONSE_HEADERS {
            return Err(OpenApiToolkitError::ResponseTooLarge {
                limit: limits.header_bytes,
            });
        }
        let header_bytes = headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.as_bytes().len())
            .sum::<usize>();
        if header_bytes > limits.header_bytes {
            return Err(OpenApiToolkitError::ResponseTooLarge {
                limit: limits.header_bytes,
            });
        }
        let status = response.status().as_u16();
        let body = TransportBody {
            inner: Box::pin(response.bytes_stream()),
        };
        Ok(HttpResponse::new(status, headers, Box::pin(body)))
    }
}

struct TransportBody {
    inner: Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>,
}

impl Stream for TransportBody {
    type Item = Result<Bytes, OpenApiToolkitError>;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(context).map(|item| {
            item.map(|result| {
                result.map_err(|_| OpenApiToolkitError::Transport("response body failed".into()))
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use http::HeaderMap;
    use secrecy::SecretString;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use url::Url;

    use super::*;
    use crate::toolkits::openapi::{AuthorizedOrigin, HttpOrigin};

    #[tokio::test]
    async fn uses_pinned_address_preserves_host_disables_redirects_and_streams() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = vec![0; 2048];
            let size = stream.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(
                request
                    .to_ascii_lowercase()
                    .contains(&format!("host: pinned.invalid:{port}"))
            );
            stream.write_all(b"HTTP/1.1 302 Found\r\nLocation: http://evil.invalid/\r\nContent-Length: 5\r\n\r\nhello").await.unwrap();
        });
        let url = Url::parse(&format!("http://pinned.invalid:{port}/resource")).unwrap();
        let origin = HttpOrigin::try_from_url(&url).unwrap();
        let authorized =
            AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]).unwrap();
        let request = HttpRequest::new(
            "GET".into(),
            url,
            HeaderMap::<SecretString>::with_capacity(0),
            None,
            authorized,
        );
        let response = ReqwestTransport::new()
            .execute(request, HttpLimits::default())
            .await
            .unwrap();
        assert_eq!(response.status(), 302);
        let mut body = response.into_body();
        let first = std::future::poll_fn(|cx| body.as_mut().poll_next(cx))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first, "hello");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn rejects_response_headers_over_the_limit() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).await;
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nX-Large: abcdefghijklmnop\r\nContent-Length: 0\r\n\r\n",
                )
                .await
                .unwrap();
        });
        let url = Url::parse(&format!("http://pinned.invalid:{port}/")).unwrap();
        let origin = HttpOrigin::try_from_url(&url).unwrap();
        let authorized =
            AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]).unwrap();
        let request = HttpRequest::new(
            "GET".into(),
            url,
            HeaderMap::<SecretString>::with_capacity(0),
            None,
            authorized,
        );
        let limits = HttpLimits {
            header_bytes: 8,
            ..HttpLimits::default()
        };
        assert!(matches!(
            ReqwestTransport.execute(request, limits).await,
            Err(OpenApiToolkitError::ResponseTooLarge { .. })
        ));
    }

    #[tokio::test]
    async fn rejects_http1_header_count_at_the_protocol_boundary() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            let _ = stream.read(&mut request).await;
            let mut response = String::from("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n");
            for index in 0..101 {
                response.push_str(&format!("X-Test-{index}: x\r\n"));
            }
            response.push_str("\r\n");
            stream.write_all(response.as_bytes()).await.unwrap();
        });
        let url = Url::parse(&format!("http://pinned.invalid:{port}/")).unwrap();
        let origin = HttpOrigin::try_from_url(&url).unwrap();
        let authorized =
            AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::LOCALHOST)]).unwrap();
        let request = HttpRequest::new(
            "GET".into(),
            url,
            HeaderMap::<SecretString>::with_capacity(0),
            None,
            authorized,
        );
        assert!(
            ReqwestTransport
                .execute(request, HttpLimits::default())
                .await
                .is_err()
        );
    }
}
