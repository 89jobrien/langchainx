/// Conformance tests for the `Chain` trait contract.
///
/// Every `Chain` impl must satisfy:
/// 1. `call()` with valid PromptArgs returns Ok(GenerateResult).
/// 2. `invoke()` returns just the generation string from `call()`.
/// 3. `execute()` returns a HashMap containing "output" and "generate_result" keys.
/// 4. `get_output_keys()` returns at least one key.
/// 5. Missing required input variables produces an error.
mod common;

use langchainx::{
    chain::LLMChainBuilder, prompt::HumanMessagePromptTemplate, prompt_args, template_fstring,
};
use langchainx_testsuite::contracts::chain::{
    assert_call_generation, assert_execute_output, assert_invoke_generation, assert_missing_input,
    assert_output_keys,
};

use common::FakeLLM;

#[tokio::test]
async fn llm_chain_missing_input_variable_returns_err() {
    let llm = FakeLLM::new(vec!["unused"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!("{a} and {b}", "a", "b"));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_missing_input(&chain, prompt_args! { "a" => "alpha" }).await;
}

#[tokio::test]
async fn llm_chain_call_returns_generate_result() {
    let llm = FakeLLM::new(vec!["Paris"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!(
        "What is the capital of {country}?",
        "country"
    ));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_call_generation(&chain, prompt_args! { "country" => "France" }, "Paris").await;
}

#[tokio::test]
async fn llm_chain_invoke_returns_string() {
    let llm = FakeLLM::new(vec!["Berlin"]);
    let prompt =
        HumanMessagePromptTemplate::new(template_fstring!("Capital of {country}?", "country"));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_invoke_generation(&chain, prompt_args! { "country" => "Germany" }, "Berlin").await;
}

#[tokio::test]
async fn llm_chain_execute_returns_hashmap_with_output_keys() {
    let llm = FakeLLM::new(vec!["Tokyo"]);
    let prompt =
        HumanMessagePromptTemplate::new(template_fstring!("Capital of {country}?", "country"));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_execute_output(&chain, prompt_args! { "country" => "Japan" }, "Tokyo").await;
}

#[tokio::test]
async fn llm_chain_get_output_keys_not_empty() {
    let llm = FakeLLM::new(vec!["x"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!("{q}", "q"));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_output_keys(&chain, &["output"]);
}

#[tokio::test]
async fn llm_chain_invoke_is_consistent_with_call() {
    let llm = FakeLLM::new(vec!["alpha", "alpha"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!("{x}", "x"));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    assert_call_generation(&chain, prompt_args! { "x" => "test" }, "alpha").await;
    assert_invoke_generation(&chain, prompt_args! { "x" => "test" }, "alpha").await;
}
