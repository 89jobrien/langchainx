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
    chain::{Chain, LLMChainBuilder},
    prompt::HumanMessagePromptTemplate,
    prompt_args, template_fstring,
};

use common::FakeLLM;

#[tokio::test]
async fn llm_chain_missing_input_variable_returns_err() {
    let llm = FakeLLM::new(vec!["unused"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!(
        "{a} and {b}",
        "a",
        "b"
    ));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    let result = chain.invoke(prompt_args! { "a" => "alpha" }).await;
    assert!(
        result.is_err(),
        "invoke() with missing input variable must return Err"
    );
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

    let result = chain
        .call(prompt_args! { "country" => "France" })
        .await;
    assert!(result.is_ok(), "call() must return Ok, got: {:?}", result.err());
    assert_eq!(result.unwrap().generation, "Paris");
}

#[tokio::test]
async fn llm_chain_invoke_returns_string() {
    let llm = FakeLLM::new(vec!["Berlin"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!(
        "Capital of {country}?",
        "country"
    ));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    let result = chain
        .invoke(prompt_args! { "country" => "Germany" })
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Berlin");
}

#[tokio::test]
async fn llm_chain_execute_returns_hashmap_with_output_keys() {
    let llm = FakeLLM::new(vec!["Tokyo"]);
    let prompt = HumanMessagePromptTemplate::new(template_fstring!(
        "Capital of {country}?",
        "country"
    ));
    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(llm)
        .build()
        .expect("build chain");

    let result = chain
        .execute(prompt_args! { "country" => "Japan" })
        .await;
    assert!(result.is_ok(), "execute() must return Ok");
    let map = result.unwrap();
    assert!(
        map.contains_key("output"),
        "execute() result must contain 'output' key, got keys: {:?}",
        map.keys().collect::<Vec<_>>()
    );
    assert!(
        map.contains_key("generate_result"),
        "execute() result must contain 'generate_result' key"
    );
    assert_eq!(map["output"].as_str(), Some("Tokyo"));
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

    let keys = chain.get_output_keys();
    assert!(
        !keys.is_empty(),
        "get_output_keys() must return at least one key"
    );
    assert!(keys.contains(&"output".to_string()));
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

    let call_gen = chain
        .call(prompt_args! { "x" => "test" })
        .await
        .unwrap()
        .generation;
    let invoke_str = chain
        .invoke(prompt_args! { "x" => "test" })
        .await
        .unwrap();
    assert_eq!(
        call_gen, invoke_str,
        "invoke() must return same string as call().generation"
    );
}
