use std::sync::Arc;

use serde_json::{Value, json};

use crate::{Tool, ToolError};

use super::{
    McpClient, McpClientError, McpToolDefinition, McpToolkitLimits,
    client::validate_content_blocks,
    framing::{serialize_bounded, serialized_size_bounded},
};

pub(crate) struct McpTool {
    client: Arc<dyn McpClient>,
    definition: McpToolDefinition,
    exposed_name: String,
    limits: McpToolkitLimits,
}

impl McpTool {
    pub(crate) fn new(
        client: Arc<dyn McpClient>,
        definition: McpToolDefinition,
        exposed_name: String,
        limits: McpToolkitLimits,
    ) -> Self {
        Self {
            client,
            definition,
            exposed_name,
            limits,
        }
    }
}

impl Tool for McpTool {
    fn name(&self) -> String {
        self.exposed_name.clone()
    }

    fn description(&self) -> String {
        format!(
            "Calls the MCP tool {} and returns untrusted third-party data.",
            self.exposed_name
        )
    }

    fn parameters(&self) -> Value {
        self.definition.input_schema.clone()
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        if !input.is_object() {
            return Err(ToolError::InvalidInput("mcp_invalid_arguments".into()));
        }
        serialized_size_bounded(&input, self.limits.argument_bytes).map_err(
            |error| match error {
                McpClientError::MessageTooLarge { .. } => {
                    ToolError::InvalidInput("mcp_arguments_too_large".into())
                }
                _ => ToolError::InvalidInput("mcp_invalid_arguments".into()),
            },
        )?;
        let result = self
            .client
            .call_tool(&self.definition.name, input)
            .await
            .map_err(|_| ToolError::ExecutionFailed("mcp_call_failed".into()))?;
        serialized_size_bounded(&result, self.limits.result_bytes)
            .map_err(map_result_size_error)?;
        if !validate_content_blocks(&result.content) {
            return Err(ToolError::ExecutionFailed("mcp_invalid_result".into()));
        }
        let envelope = json!({
            "type": "untrusted_third_party_data",
            "data": result.content,
        });
        if result.is_error {
            serialized_size_bounded(&envelope, self.limits.result_bytes)
                .map_err(map_result_size_error)?;
            return Err(ToolError::ExecutionFailed("mcp_tool_reported_error".into()));
        }
        let output = serialize_bounded(&envelope, self.limits.result_bytes)
            .map_err(map_result_size_error)?;
        String::from_utf8(output)
            .map_err(|_| ToolError::ExecutionFailed("mcp_invalid_result".into()))
    }

    async fn parse_input(&self, input: &str) -> Value {
        if input.len() > self.limits.argument_bytes {
            return Value::Null;
        }
        serde_json::from_str(input).unwrap_or(Value::Null)
    }
}

fn map_result_size_error(error: McpClientError) -> ToolError {
    match error {
        McpClientError::MessageTooLarge { .. } => {
            ToolError::ExecutionFailed("mcp_result_too_large".into())
        }
        _ => ToolError::ExecutionFailed("mcp_invalid_result".into()),
    }
}
