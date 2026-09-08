/// Conformance tests for the `LLM` trait contract.
///
/// Every `LLM` impl must satisfy:
/// 1. `generate()` with a non-empty message list returns Ok(GenerateResult).
/// 2. `invoke()` returns the generation string from `generate()`.
/// 3. `messages_to_string()` produces a non-empty string for non-empty input.
/// 4. GenerateResult.generation is the actual content (not a wrapper/envelope).
mod common;

use langchainx::{language_models::llm::LLM, schemas::Message};
use langchainx_testsuite::contracts::llm::{
    assert_generate_contract, assert_invoke_contract, assert_message_format_contract,
    assert_stream_contract,
};

use common::FakeLLM;

#[tokio::test]
async fn fake_llm_satisfies_contract() {
    let llm = FakeLLM::new(vec!["response-1", "response-2", "response-3"]);
    assert_generate_contract(&llm, "response-1").await;
    assert_invoke_contract(&llm, "response-2").await;
    assert_stream_contract(&llm, "response-3").await;
    let messages = [
        Message::new_system_message("system"),
        Message::new_human_message("human"),
        Message::new_ai_message("ai"),
        Message::new_tool_message("tool", "call-1"),
    ];
    assert_message_format_contract(
        &llm,
        &messages,
        "SystemMessage: system\nHumanMessage: human\nAIMessage: ai\nToolMessage: tool",
    );
}

#[tokio::test]
async fn fake_llm_exhausted_returns_empty() {
    let llm = FakeLLM::new(Vec::<String>::new());
    let result = llm.generate(&[Message::new_human_message("hi")]).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().generation,
        "",
        "exhausted LLM must return empty string, not error"
    );
}
