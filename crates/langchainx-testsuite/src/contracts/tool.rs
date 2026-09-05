use std::sync::Arc;

use langchainx_core::tools::Tool;
use serde_json::Value;

/// Asserts nonempty metadata, object-schema parameters, and successful valid execution.
pub async fn assert_tool_contract(tool: Arc<dyn Tool>, valid_input: &str) {
    let name = tool.name();
    assert!(!name.is_empty(), "tool name must not be empty");
    assert!(!name.contains(' '), "tool name must not contain spaces");
    assert_eq!(tool.name(), name, "tool name must be stable across calls");
    assert!(
        !tool.description().is_empty(),
        "tool description must not be empty"
    );
    let parameters = tool.parameters();
    assert_eq!(
        parameters.get("type").and_then(Value::as_str),
        Some("object")
    );
    assert!(
        parameters.get("properties").is_some_and(Value::is_object),
        "tool parameters must contain an object-valued properties field"
    );
    tool.call(valid_input)
        .await
        .expect("Tool::call must accept valid input");
    let _ = tool.call("").await;
}

/// Asserts plain, wrapped, arbitrary, and malformed input behavior of the default parser.
pub async fn assert_default_parse_input(tool: Arc<dyn Tool>) {
    assert_eq!(
        tool.parse_input("plain text").await,
        Value::String("plain text".into())
    );
    assert_eq!(
        tool.parse_input(r#"{"input":"hello"}"#).await,
        Value::String("hello".into())
    );
    assert_eq!(
        tool.parse_input(r#"{"other":42}"#).await,
        Value::String(r#"{"other":42}"#.into())
    );
    assert_eq!(
        tool.parse_input("{malformed").await,
        Value::String("{malformed".into())
    );
}
