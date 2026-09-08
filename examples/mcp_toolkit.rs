//! Discovers generated MCP tools from an in-memory client with no subprocess or network.

use std::sync::Arc;

use async_trait::async_trait;
use langchainx::tools::toolkits::mcp::{
    McpClient, McpClientError, McpToolDefinition, McpToolPage, McpToolResult, McpToolkit,
    McpToolkitLimits,
};
use serde_json::{Value, json};

#[derive(Debug, Default)]
struct MemoryMcpClient;

#[async_trait]
impl McpClient for MemoryMcpClient {
    async fn list_tools(&self, _cursor: Option<String>) -> Result<McpToolPage, McpClientError> {
        Ok(McpToolPage {
            tools: vec![McpToolDefinition {
                name: "lookup_item".into(),
                description: Some("Looks up an inventory item".into()),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "item_id": { "type": "string" }
                    },
                    "required": ["item_id"]
                }),
            }],
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpToolResult, McpClientError> {
        Ok(McpToolResult {
            content: json!([{
                "type": "text",
                "text": format!("in-memory call to {name} with {arguments}")
            }]),
            is_error: false,
        })
    }

    async fn close(&self) -> Result<(), McpClientError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let toolkit = McpToolkit::new(Arc::new(MemoryMcpClient), McpToolkitLimits::default()).await?;

    for tool in toolkit.tools() {
        println!("generated MCP tool: {}", tool.name());
    }

    toolkit.close().await?;
    Ok(())
}
