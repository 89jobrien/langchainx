/// Conformance tests for the `Tool` trait contract.
///
/// Every `Tool` impl must satisfy:
/// 1. `name()` returns a non-empty string.
/// 2. `description()` returns a non-empty string.
/// 3. `parameters()` returns valid JSON with "type": "object".
/// 4. `call()` with valid input returns Ok.
use std::sync::Arc;

use serde_json::Value;

use langchainx::tools::{Tool, ToolError};

async fn assert_tool_contract(tool: Arc<dyn Tool>, valid_input: &str) {
    // 1. Non-empty name
    let name = tool.name();
    assert!(!name.is_empty(), "tool name must not be empty");

    // 2. Non-empty description
    let desc = tool.description();
    assert!(!desc.is_empty(), "tool description must not be empty");

    // 3. Parameters is a JSON object
    let params = tool.parameters();
    assert_eq!(
        params.get("type").and_then(|v| v.as_str()),
        Some("object"),
        "parameters().type must be 'object', got: {params}"
    );

    // 4. call() with valid input succeeds
    let result = tool.call(valid_input).await;
    assert!(
        result.is_ok(),
        "call({valid_input:?}) should succeed, got: {:?}",
        result.err()
    );
}

/// Default parse_input behavior — tested separately since impls may override.
async fn assert_default_parse_input(tool: Arc<dyn Tool>) {
    let parsed = tool.parse_input("plain text").await;
    assert!(
        parsed.is_string(),
        "default parse_input on plain text should yield Value::String"
    );

    let parsed = tool.parse_input(r#"{"input": "hello"}"#).await;
    assert_eq!(parsed.as_str(), Some("hello"));
}

/// A minimal tool for testing the contract without side effects.
struct NoopTool;

#[async_trait::async_trait]
impl Tool for NoopTool {
    fn name(&self) -> String {
        "noop".into()
    }
    fn description(&self) -> String {
        "Does nothing".into()
    }
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        Ok(format!("got: {input}"))
    }
}

#[tokio::test]
async fn noop_tool_satisfies_contract() {
    let tool = Arc::new(NoopTool);
    assert_tool_contract(tool.clone(), "test input").await;
    assert_default_parse_input(tool).await;
}

#[tokio::test]
async fn command_executor_satisfies_contract() {
    let tool = langchainx::tools::CommandExecutor::new("bash");
    let input = r#"[{"cmd": "echo", "args": ["hello"]}]"#;
    assert_tool_contract(Arc::new(tool), input).await;
}
