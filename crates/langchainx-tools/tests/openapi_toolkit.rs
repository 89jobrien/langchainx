#![cfg(feature = "openapi-toolkit")]

use std::{
    net::{IpAddr, Ipv4Addr},
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
    time::Duration,
};

use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use http::HeaderMap;
use langchainx_testsuite::contracts::tool::assert_tool_contract;
use langchainx_tools::ToolError;
use langchainx_tools::toolkits::openapi::{
    AllowAllOperations, AuthorizedOrigin, HttpBody, HttpEgressPolicy, HttpOrigin, HttpRequest,
    HttpResponse, HttpTransport, OpenApiFormat, OpenApiLimits, OpenApiToolkitBuilder,
    OpenApiToolkitError, ReadOnlyOperations,
};
use url::Url;

#[derive(Default)]
struct MemoryTransport {
    requests: Mutex<Vec<HttpRequest>>,
}

#[async_trait]
impl HttpTransport for MemoryTransport {
    async fn execute(
        &self,
        request: HttpRequest,
        _limits: langchainx_tools::toolkits::openapi::HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        self.requests.lock().unwrap().push(request);
        let body: HttpBody = Box::pin(OneShot(Some(Bytes::from_static(br#"{"ok":true}"#))));
        Ok(HttpResponse::new(200, HeaderMap::new(), body))
    }
}

struct OneShot(Option<Bytes>);
impl Stream for OneShot {
    type Item = Result<Bytes, OpenApiToolkitError>;
    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take().map(Ok))
    }
}

struct LocalPolicy;
#[async_trait]
impl HttpEgressPolicy for LocalPolicy {
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError> {
        AuthorizedOrigin::new(
            HttpOrigin::try_from_url(url)?,
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        )
    }
}

struct SlowPolicy;

#[async_trait]
impl HttpEgressPolicy for SlowPolicy {
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError> {
        tokio::time::sleep(Duration::from_millis(25)).await;
        LocalPolicy.authorize(url).await
    }
}

struct FailureTransport;

#[async_trait]
impl HttpTransport for FailureTransport {
    async fn execute(
        &self,
        _request: HttpRequest,
        _limits: langchainx_tools::toolkits::openapi::HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        let body: HttpBody = Box::pin(OneShot(Some(Bytes::from_static(b"upstream failure"))));
        Ok(HttpResponse::new(503, HeaderMap::new(), body))
    }
}

struct LeakyTransport;

#[async_trait]
impl HttpTransport for LeakyTransport {
    async fn execute(
        &self,
        _request: HttpRequest,
        _limits: langchainx_tools::toolkits::openapi::HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        Err(OpenApiToolkitError::Transport(
            "must-not-leak transport detail".into(),
        ))
    }
}

struct InvalidUtf8Transport;

#[async_trait]
impl HttpTransport for InvalidUtf8Transport {
    async fn execute(
        &self,
        _request: HttpRequest,
        _limits: langchainx_tools::toolkits::openapi::HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        let body: HttpBody = Box::pin(OneShot(Some(Bytes::from_static(&[0xff, 0xfe]))));
        Ok(HttpResponse::new(200, HeaderMap::new(), body))
    }
}

const SPEC: &str = r#"{"openapi":"3.0.0","servers":[{"url":"http://service.test"}],"paths":{"/items":{"get":{"operationId":"listItems","responses":{}},"post":{"operationId":"createItem","requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object"}}}},"responses":{}}}}}"#;

#[tokio::test]
async fn builds_all_methods_preserves_structured_input_and_wraps_untrusted_output() {
    let transport = Arc::new(MemoryTransport::default());
    let toolkit = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        transport.clone(),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()
    .unwrap();
    let tools = toolkit.tools();
    assert_eq!(tools.len(), 2);
    let get = tools
        .iter()
        .find(|tool| tool.name() == "list_items")
        .unwrap();
    assert_eq!(
        get.parse_input(r#"{"query":{}}"#).await,
        serde_json::json!({"query":{}})
    );
    let output = get.run(serde_json::json!({})).await.unwrap();
    let output: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(output["type"], "untrusted_third_party_data");
    assert_eq!(output["data"]["ok"], true);
    assert_eq!(transport.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn generated_get_and_post_tools_satisfy_the_shared_contract() {
    let toolkit = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport::default()),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()
    .unwrap();
    let tools = toolkit.tools();
    let get = tools
        .iter()
        .find(|tool| tool.name() == "list_items")
        .unwrap()
        .clone();
    let post = tools
        .iter()
        .find(|tool| tool.name() == "create_item")
        .unwrap()
        .clone();
    assert_tool_contract(get, "{}").await;
    assert_tool_contract(post, r#"{"body":{}}"#).await;
}

#[test]
fn requires_policy_and_binds_credentials_to_exact_operation_origins() {
    let base = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport::default()),
        Arc::new(LocalPolicy),
    );
    assert!(matches!(
        base.build(),
        Err(OpenApiToolkitError::MissingOperationPolicy)
    ));
    let origin = HttpOrigin::try_from_url(&Url::parse("https://other.test").unwrap()).unwrap();
    let builder = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport::default()),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .with_credential_header(
        origin,
        http::HeaderName::from_static("x-secret-token"),
        secrecy::SecretString::from("secret"),
    )
    .unwrap();
    assert!(builder.build().is_err());

    let origin = HttpOrigin::try_from_url(&Url::parse("http://service.test").unwrap()).unwrap();
    for name in ["authorization", "x-api-key"] {
        assert!(
            OpenApiToolkitBuilder::new(
                SPEC,
                OpenApiFormat::Json,
                Arc::new(MemoryTransport::default()),
                Arc::new(LocalPolicy),
            )
            .with_credential_header(
                origin.clone(),
                http::HeaderName::from_static(name),
                secrecy::SecretString::from("trusted-secret"),
            )
            .is_ok()
        );
    }
    for name in ["proxy-authorization", "host", "transfer-encoding"] {
        assert!(
            OpenApiToolkitBuilder::new(
                SPEC,
                OpenApiFormat::Json,
                Arc::new(MemoryTransport::default()),
                Arc::new(LocalPolicy),
            )
            .with_credential_header(
                origin.clone(),
                http::HeaderName::from_static(name),
                secrecy::SecretString::from("must-not-leak"),
            )
            .is_err()
        );
    }

    let agent_override = r#"{"openapi":"3.0.0","servers":[{"url":"http://service.test"}],"paths":{"/x":{"get":{"operationId":"x","parameters":[{"name":"Authorization","in":"header","schema":{"type":"string"}}],"responses":{}}}}}"#;
    assert!(
        OpenApiToolkitBuilder::new(
            agent_override,
            OpenApiFormat::Json,
            Arc::new(MemoryTransport::default()),
            Arc::new(LocalPolicy),
        )
        .with_operation_policy(Arc::new(AllowAllOperations))
        .build()
        .is_err()
    );
}

#[tokio::test]
async fn enforces_read_only_policy_shared_timeout_and_untrusted_error_envelope() {
    let read_only = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport::default()),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(ReadOnlyOperations))
    .build()
    .unwrap();
    let post = read_only
        .tools()
        .into_iter()
        .find(|tool| tool.name() == "create_item")
        .unwrap();
    assert!(matches!(
        post.run(serde_json::json!({"body":{}})).await,
        Err(ToolError::ExecutionFailed(_))
    ));

    let mut limits = OpenApiLimits::default();
    limits.http.timeout = Duration::from_millis(1);
    let timed = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport::default()),
        Arc::new(SlowPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .with_limits(limits)
    .build()
    .unwrap();
    let get = timed.tools().remove(0);
    assert!(matches!(
        get.run(serde_json::json!({})).await,
        Err(ToolError::ExecutionFailed(message)) if message == "openapi_operation_timed_out"
    ));

    let failed = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(FailureTransport),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()
    .unwrap();
    let get = failed.tools().remove(0);
    assert!(matches!(
        get.run(serde_json::json!({})).await,
        Err(ToolError::ExecutionFailed(message))
            if message == "openapi_http_status:503"
    ));
}

#[tokio::test]
async fn rejects_oversized_raw_input_without_copying_and_redacts_arbitrary_errors() {
    let mut limits = OpenApiLimits::default();
    limits.http.request_body_bytes = 32;
    let toolkit = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(FailureTransport),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .with_limits(limits)
    .build()
    .unwrap();
    let tool = toolkit.tools().remove(0);
    let oversized = "x".repeat(1_024);
    assert_eq!(tool.parse_input(&oversized).await, serde_json::Value::Null);
    assert!(matches!(
        tool.run(serde_json::json!({"query":{"secret":"must-not-leak"}})).await,
        Err(ToolError::InvalidInput(message)) if !message.contains("must-not-leak")
    ));

    let toolkit = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(LeakyTransport),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()
    .unwrap();
    let tool = toolkit.tools().remove(0);
    assert!(matches!(
        tool.run(serde_json::json!({})).await,
        Err(ToolError::ExecutionFailed(message))
            if message == "openapi_transport_failed" && !message.contains("must-not-leak")
    ));

    let toolkit = OpenApiToolkitBuilder::new(
        SPEC,
        OpenApiFormat::Json,
        Arc::new(InvalidUtf8Transport),
        Arc::new(LocalPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()
    .unwrap();
    let tool = toolkit.tools().remove(0);
    assert!(matches!(
        tool.run(serde_json::json!({})).await,
        Err(ToolError::ExecutionFailed(message)) if message == "openapi_invalid_response_encoding"
    ));
}
