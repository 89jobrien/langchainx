use futures_util::StreamExt;
use langchainx_core::schemas::Message;
use langchainx_llm::language_models::llm::LLM;

/// Asserts that generation succeeds and exposes the expected unwrapped text.
pub async fn assert_generate_contract(llm: &dyn LLM, expected: &str) {
    let messages = [Message::new_human_message("test prompt")];
    let result = llm
        .generate(&messages)
        .await
        .expect("LLM::generate must succeed");
    assert_eq!(result.generation, expected);
}

/// Asserts that the convenience invocation returns exactly the generated text.
pub async fn assert_invoke_contract(llm: &dyn LLM, expected: &str) {
    let result = llm.invoke("test").await.expect("LLM::invoke must succeed");
    assert_eq!(result, expected);
}

/// Asserts that successful stream chunks combine into the expected content.
pub async fn assert_stream_contract(llm: &dyn LLM, expected: &str) {
    let messages = [Message::new_human_message("test prompt")];
    let mut stream = llm
        .stream(&messages)
        .await
        .expect("LLM::stream must succeed");
    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        content.push_str(&chunk.expect("LLM stream chunk must be successful").content);
    }
    assert_eq!(content, expected);
}

/// Asserts the exact role-prefixed representation used for non-chat models.
pub fn assert_message_format_contract(llm: &dyn LLM, messages: &[Message], expected: &str) {
    assert_eq!(llm.messages_to_string(messages), expected);
}
