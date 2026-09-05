//! Builds OpenAPI tools with in-memory HTTP ports and no external I/O.

use std::{
    net::{IpAddr, Ipv4Addr},
    sync::Arc,
};

use async_trait::async_trait;
use langchainx::tools::toolkits::openapi::{
    AllowAllOperations, AuthorizedOrigin, HttpBody, HttpEgressPolicy, HttpLimits, HttpOrigin,
    HttpRequest, HttpResponse, HttpTransport, OpenApiFormat, OpenApiToolkitBuilder,
    OpenApiToolkitError,
};
use langchainx::url::Url;

const SPECIFICATION: &str = r#"
{
  "openapi": "3.0.0",
  "servers": [{ "url": "http://inventory.example" }],
  "paths": {
    "/items": {
      "get": {
        "operationId": "listItems",
        "responses": { "200": { "description": "Items" } }
      }
    }
  }
}
"#;

#[derive(Debug, Default)]
struct MemoryTransport;

#[async_trait]
impl HttpTransport for MemoryTransport {
    async fn execute(
        &self,
        _request: HttpRequest,
        _limits: HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError> {
        let body: HttpBody = Box::pin(futures::stream::empty());
        Ok(HttpResponse::new(200, Default::default(), body))
    }
}

#[derive(Debug, Default)]
struct MemoryEgressPolicy;

#[async_trait]
impl HttpEgressPolicy for MemoryEgressPolicy {
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError> {
        AuthorizedOrigin::new(
            HttpOrigin::try_from_url(url)?,
            vec![IpAddr::V4(Ipv4Addr::LOCALHOST)],
        )
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let toolkit = OpenApiToolkitBuilder::new(
        SPECIFICATION,
        OpenApiFormat::Json,
        Arc::new(MemoryTransport),
        Arc::new(MemoryEgressPolicy),
    )
    .with_operation_policy(Arc::new(AllowAllOperations))
    .build()?;

    for tool in toolkit.tools() {
        println!("generated OpenAPI tool: {}", tool.name());
    }

    Ok(())
}
