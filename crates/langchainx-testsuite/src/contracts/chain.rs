use langchainx_chain::Chain;
use langchainx_prompt::PromptArgs;

/// Asserts that `call` succeeds with exactly the expected generation.
pub async fn assert_call_generation(chain: &dyn Chain, inputs: PromptArgs, expected: &str) {
    let result = chain.call(inputs).await.expect("Chain::call must succeed");
    assert_eq!(result.generation, expected);
}

/// Asserts that `invoke` succeeds with exactly the expected generation.
pub async fn assert_invoke_generation(chain: &dyn Chain, inputs: PromptArgs, expected: &str) {
    let result = chain
        .invoke(inputs)
        .await
        .expect("Chain::invoke must succeed");
    assert_eq!(result, expected);
}

/// Asserts that `execute` exposes text under `output` and the complete generation result.
pub async fn assert_execute_output(chain: &dyn Chain, inputs: PromptArgs, expected: &str) {
    let output = chain
        .execute(inputs)
        .await
        .expect("Chain::execute must succeed");
    assert_eq!(
        output.get("output").and_then(|value| value.as_str()),
        Some(expected)
    );
    assert!(
        output.contains_key("generate_result"),
        "Chain::execute must include generate_result"
    );
}

/// Asserts that inputs missing a required key are rejected rather than panicking.
pub async fn assert_missing_input(chain: &dyn Chain, inputs: PromptArgs) {
    assert!(
        chain.invoke(inputs).await.is_err(),
        "Chain::invoke must reject missing input"
    );
}

/// Asserts that output keys exactly match the implementation's declared contract.
pub fn assert_output_keys(chain: &dyn Chain, expected: &[&str]) {
    let actual = chain.get_output_keys();
    assert!(!actual.is_empty(), "Chain output keys must not be empty");
    assert_eq!(actual, expected);
}
