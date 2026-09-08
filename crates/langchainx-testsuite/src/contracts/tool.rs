use std::sync::Arc;

use langchainx_core::tools::DynTool;
use serde_json::Value;

/// Asserts nonempty metadata, object-schema parameters, and successful valid execution.
pub async fn assert_tool_contract(tool: Arc<dyn DynTool>, valid_input: &str) {
    let name = tool.dyn_name();
    assert!(!name.is_empty(), "tool name must not be empty");
    assert!(!name.contains(' '), "tool name must not contain spaces");
    assert_eq!(
        tool.dyn_name(),
        name,
        "tool name must be stable across calls"
    );
    assert!(
        !tool.dyn_description().is_empty(),
        "tool description must not be empty"
    );
    let parameters = tool.dyn_parameters();
    assert_eq!(
        parameters.get("type").and_then(Value::as_str),
        Some("object")
    );
    assert!(
        parameters.get("properties").is_some_and(Value::is_object),
        "tool parameters must contain an object-valued properties field"
    );
    tool.dyn_call(valid_input)
        .await
        .expect("Tool::call must accept valid input");
    let _ = tool.dyn_call("").await;
}

/// Asserts plain, wrapped, arbitrary, and malformed input behavior of the default parser.
pub async fn assert_default_parse_input(tool: Arc<dyn DynTool>) {
    assert_eq!(
        tool.dyn_parse_input("plain text").await,
        Value::String("plain text".into())
    );
    assert_eq!(
        tool.dyn_parse_input(r#"{"input":"hello"}"#).await,
        Value::String("hello".into())
    );
    assert_eq!(
        tool.dyn_parse_input(r#"{"other":42}"#).await,
        Value::String(r#"{"other":42}"#.into())
    );
    assert_eq!(
        tool.dyn_parse_input("{malformed").await,
        Value::String("{malformed".into())
    );
}
