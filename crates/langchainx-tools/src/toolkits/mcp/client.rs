use std::fmt::{self, Debug, Formatter};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::McpClientError;

/// A discovered MCP tool and its input schema.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDefinition {
    /// Server-defined tool name used for protocol calls.
    pub name: String,
    /// Untrusted server description, retained for protocol fidelity but not exposed to models.
    #[serde(default)]
    pub description: Option<String>,
    /// JSON Schema describing the tool's argument object.
    pub input_schema: Value,
}

/// One page returned by MCP tool discovery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolPage {
    /// Tool definitions in this page.
    pub tools: Vec<McpToolDefinition>,
    /// Opaque cursor for the next page.
    #[serde(default)]
    pub next_cursor: Option<String>,
}

/// Content returned by an MCP tool call.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolResult {
    /// Untrusted result content.
    pub content: Value,
    /// Whether the server classified the result as an application error.
    #[serde(default)]
    pub is_error: bool,
}

impl Debug for McpToolResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let content_blocks = self.content.as_array().map_or(0, Vec::len);
        formatter
            .debug_struct("McpToolResult")
            .field("content_blocks", &content_blocks)
            .field("is_error", &self.is_error)
            .finish()
    }
}

pub(crate) fn validate_content_blocks(content: &Value) -> bool {
    content
        .as_array()
        .is_some_and(|blocks| blocks.iter().all(valid_content_block))
}

fn valid_content_block(block: &Value) -> bool {
    let Some(object) = block.as_object() else {
        return false;
    };
    if !valid_optional_metadata(object) {
        return false;
    }
    match object.get("type").and_then(Value::as_str) {
        Some("text") => object.get("text").is_some_and(Value::is_string),
        Some("image" | "audio") => {
            object.get("data").is_some_and(Value::is_string)
                && object.get("mimeType").is_some_and(Value::is_string)
        }
        Some("resource") => object.get("resource").is_some_and(valid_embedded_resource),
        Some("resource_link") => {
            object.get("uri").is_some_and(Value::is_string)
                && object.get("name").is_some_and(Value::is_string)
                && object.get("mimeType").is_none_or(Value::is_string)
                && object
                    .get("size")
                    .is_none_or(|size| size.as_u64().is_some())
        }
        _ => false,
    }
}

fn valid_embedded_resource(resource: &Value) -> bool {
    let Some(resource) = resource.as_object() else {
        return false;
    };
    resource.get("uri").is_some_and(Value::is_string)
        && resource.get("mimeType").is_none_or(Value::is_string)
        && matches!(
            (
                resource.get("text").is_some_and(Value::is_string),
                resource.get("blob").is_some_and(Value::is_string),
            ),
            (true, false) | (false, true)
        )
}

fn valid_optional_metadata(object: &serde_json::Map<String, Value>) -> bool {
    object.get("annotations").is_none_or(valid_annotations)
        && object.get("_meta").is_none_or(Value::is_object)
}

fn valid_annotations(value: &Value) -> bool {
    let Some(annotations) = value.as_object() else {
        return false;
    };
    annotations.get("audience").is_none_or(|audience| {
        audience.as_array().is_some_and(|roles| {
            roles
                .iter()
                .all(|role| matches!(role.as_str(), Some("user" | "assistant")))
        })
    }) && annotations.get("priority").is_none_or(|priority| {
        priority
            .as_f64()
            .is_some_and(|priority| (0.0..=1.0).contains(&priority))
    }) && annotations.get("lastModified").is_none_or(Value::is_string)
}

/// Abstract MCP connection used by discovery and generated tools.
#[async_trait]
pub trait McpClient: Send + Sync {
    /// Lists one page of server tools starting at an optional opaque cursor.
    ///
    /// # Errors
    ///
    /// Returns [`McpClientError`] when transport, protocol, framing, or timeout handling fails.
    async fn list_tools(&self, cursor: Option<String>) -> Result<McpToolPage, McpClientError>;

    /// Calls a server tool with a structured argument value.
    ///
    /// # Errors
    ///
    /// Returns [`McpClientError`] when the request cannot be sent or its response is invalid.
    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpToolResult, McpClientError>;

    /// Closes the connection. Implementations must permit repeated calls.
    ///
    /// # Errors
    ///
    /// Returns [`McpClientError`] when bounded shutdown or process cleanup fails.
    async fn close(&self) -> Result<(), McpClientError>;
}
