/// Conformance tests for the `LLM` trait contract.
///
/// Every `LLM` impl must satisfy:
/// 1. `generate()` with a non-empty message list returns Ok(GenerateResult).
/// 2. `invoke()` returns the generation string from `generate()`.
/// 3. `messages_to_string()` produces a non-empty string for non-empty input.
/// 4. GenerateResult.generation is the actual content (not a wrapper/envelope).
mod common;

use langchainx::{language_models::llm::LLM, schemas::Message};

use common::FakeLLM;

async fn assert_llm_contract(llm: &dyn LLM, expected_generation: &str) {
    // 1. generate() returns Ok
    let messages = vec![Message::new_human_message("test prompt")];
    let result = llm.generate(&messages).await;
    assert!(
        result.is_ok(),
        "generate() must return Ok, got: {:?}",
        result.err()
    );
    let gen_result = result.unwrap();

    // 2. generation matches expected
    assert_eq!(
        gen_result.generation, expected_generation,
        "generation content mismatch"
    );

    // 3. messages_to_string produces non-empty output
    let s = llm.messages_to_string(&messages);
    assert!(!s.is_empty(), "messages_to_string must not be empty");
    assert!(
        s.contains("test prompt"),
        "messages_to_string must contain the message content"
    );
}

/// invoke() is a convenience wrapper — verify it returns just the string.
async fn assert_llm_invoke_contract(llm: &dyn LLM, expected: &str) {
    let result = llm.invoke("test").await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected);
}

#[tokio::test]
async fn fake_llm_satisfies_contract() {
    let llm = FakeLLM::new(vec!["response-1", "response-2"]);
    assert_llm_contract(&llm, "response-1").await;
    assert_llm_invoke_contract(&llm, "response-2").await;
}

#[tokio::test]
async fn fake_llm_exhausted_returns_empty() {
    let llm = FakeLLM::new(vec![]);
    let result = llm.generate(&[Message::new_human_message("hi")]).await;
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap().generation,
        "",
        "exhausted LLM must return empty string, not error"
    );
}
