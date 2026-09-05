/// Conformance tests for the `Tool` trait contract.
///
/// Every `Tool` impl must satisfy:
/// 1. `name()` returns a non-empty string.
/// 2. `description()` returns a non-empty string.
/// 3. `parameters()` returns valid JSON with "type": "object".
/// 4. `call()` with valid input returns Ok.
use std::sync::Arc;

use langchainx_testsuite::{
    contracts::tool::{assert_default_parse_input, assert_tool_contract},
    fakes::EchoTool,
};

#[tokio::test]
async fn echo_tool_satisfies_contract() {
    let tool = Arc::new(EchoTool);
    assert_tool_contract(tool.clone(), "test input").await;
    assert_default_parse_input(tool).await;
}

#[tokio::test]
async fn command_executor_satisfies_contract() {
    let tool = langchainx::tools::CommandExecutor::new("bash");
    let input = r#"[{"cmd": "echo", "args": ["hello"]}]"#;
    assert_tool_contract(Arc::new(tool), input).await;
}
