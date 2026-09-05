use std::sync::Arc;

use async_trait::async_trait;
use http::HeaderMap;
use secrecy::SecretString;
use serde_json::{Value, json};

use crate::{Tool, ToolError};

use super::{
    HttpEgressPolicy, HttpTransport, OpenApiLimits, OpenApiToolkitError, OperationPolicy,
    arguments::prepare_request, operation::OpenApiOperation,
};

pub(crate) struct OpenApiTool {
    operation: OpenApiOperation,
    transport: Arc<dyn HttpTransport>,
    egress_policy: Arc<dyn HttpEgressPolicy>,
    operation_policy: Arc<dyn OperationPolicy>,
    credential_headers: HeaderMap<SecretString>,
    limits: OpenApiLimits,
}

impl OpenApiTool {
    pub(crate) fn new(
        operation: OpenApiOperation,
        transport: Arc<dyn HttpTransport>,
        egress_policy: Arc<dyn HttpEgressPolicy>,
        operation_policy: Arc<dyn OperationPolicy>,
        credential_headers: HeaderMap<SecretString>,
        limits: OpenApiLimits,
    ) -> Self {
        Self {
            operation,
            transport,
            egress_policy,
            operation_policy,
            credential_headers,
            limits,
        }
    }

    async fn execute(&self, input: Value) -> Result<String, OpenApiToolkitError> {
        self.operation_policy
            .authorize(&self.operation.context())
            .await?;
        let prepared = prepare_request(
            &self.operation,
            &input,
            &self.credential_headers,
            self.limits.http,
        )?;
        let authorized = self.egress_policy.authorize(prepared.url()).await?;
        let request = prepared.authorize(authorized)?;
        let response = self.transport.execute(request, self.limits.http).await?;
        let status = response.status();
        let mut body_stream = response.into_body();
        let mut body = Vec::new();
        let mut body_bytes = 0usize;
        while let Some(chunk) =
            std::future::poll_fn(|context| body_stream.as_mut().poll_next(context)).await
        {
            let chunk = chunk?;
            body_bytes = body_bytes.saturating_add(chunk.len());
            if body_bytes > self.limits.http.response_body_bytes {
                return Err(OpenApiToolkitError::ResponseTooLarge {
                    limit: self.limits.http.response_body_bytes,
                });
            }
            if (200..300).contains(&status) {
                body.extend_from_slice(&chunk);
            }
        }
        if !(200..300).contains(&status) {
            return Err(OpenApiToolkitError::Transport(format!(
                "http_status:{status}"
            )));
        }
        let data =
            match serde_json::from_slice(&body) {
                Ok(data) => data,
                Err(_) => Value::String(String::from_utf8(body).map_err(|_| {
                    OpenApiToolkitError::Transport("invalid_response_encoding".into())
                })?),
            };
        Ok(json!({
            "type": "untrusted_third_party_data",
            "status": status,
            "data": data,
        })
        .to_string())
    }
}

#[async_trait]
impl Tool for OpenApiTool {
    fn name(&self) -> String {
        self.operation.exposed_name.clone()
    }

    fn description(&self) -> String {
        self.operation.description()
    }

    fn parameters(&self) -> Value {
        self.operation.tool_schema.clone()
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        tokio::time::timeout(self.limits.http.timeout, self.execute(input))
            .await
            .map_err(|_| ToolError::ExecutionFailed("openapi_operation_timed_out".into()))?
            .map_err(map_tool_error)
    }

    async fn parse_input(&self, input: &str) -> Value {
        if input.len() > self.limits.http.request_body_bytes {
            return Value::Null;
        }
        serde_json::from_str(input).unwrap_or_else(|_| Value::String(input.to_owned()))
    }
}

fn map_tool_error(error: OpenApiToolkitError) -> ToolError {
    match error {
        OpenApiToolkitError::InvalidArguments(_)
        | OpenApiToolkitError::ProtectedHeader(_)
        | OpenApiToolkitError::RequestTooLarge { .. } => {
            ToolError::InvalidInput("openapi_invalid_input".into())
        }
        OpenApiToolkitError::Transport(message) => {
            if message == "invalid_response_encoding" {
                return ToolError::ExecutionFailed("openapi_invalid_response_encoding".into());
            }
            let status = message
                .strip_prefix("http_status:")
                .and_then(|status| status.parse::<u16>().ok());
            match status {
                Some(status) => ToolError::ExecutionFailed(format!("openapi_http_status:{status}")),
                None => ToolError::ExecutionFailed("openapi_transport_failed".into()),
            }
        }
        OpenApiToolkitError::MutatingOperationDenied(_) => {
            ToolError::ExecutionFailed("openapi_operation_denied".into())
        }
        OpenApiToolkitError::EgressDenied(_) => {
            ToolError::ExecutionFailed("openapi_egress_denied".into())
        }
        OpenApiToolkitError::ResponseTooLarge { .. } => {
            ToolError::ExecutionFailed("openapi_response_too_large".into())
        }
        OpenApiToolkitError::Timeout => {
            ToolError::ExecutionFailed("openapi_operation_timed_out".into())
        }
        _ => ToolError::ExecutionFailed("openapi_execution_failed".into()),
    }
}
