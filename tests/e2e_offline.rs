/// Tier 1 — offline e2e tests.
///
/// All tests use FakeLLM and FakeEmbedder. No network, no model, no containers.
/// These must always pass in CI.
mod common;

use std::sync::Arc;

use langchainx::{
    chain::{
        Chain, LLMChainBuilder, SequentialChainBuilder,
        conversational::builder::ConversationalChainBuilder,
    },
    fmt_template,
    memory::SimpleMemory,
    message_formatter,
    prompt::HumanMessagePromptTemplate,
    prompt_args, template_fstring,
    tools::{Tool, ToolError},
};

use common::FakeLLM;

// ---------------------------------------------------------------------------
// LLMChain — basic invoke
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_llm_chain_invoke() {
    let fake = FakeLLM::new(vec!["Hello from FakeLLM!"]);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(fake.clone())
        .prompt(prompt)
        .build()
        .unwrap();

    let result = chain
        .invoke(prompt_args! { "input" => "Hi" })
        .await
        .unwrap();

    assert_eq!(result, "Hello from FakeLLM!");
    assert_eq!(fake.call_count(), 1);
}

// ---------------------------------------------------------------------------
// LLMChain — empty response queue returns empty string (not a panic)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_llm_chain_empty_response_queue() {
    let fake = FakeLLM::new(vec![]);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(fake)
        .prompt(prompt)
        .build()
        .unwrap();

    let result = chain
        .invoke(prompt_args! { "input" => "Hi" })
        .await
        .unwrap();

    assert_eq!(result, "", "exhausted FakeLLM should return empty string");
}

// ---------------------------------------------------------------------------
// LLMChain — multiple sequential invocations consume responses in order
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_llm_chain_multiple_invocations() {
    let fake = FakeLLM::new(vec!["first", "second", "third"]);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(fake.clone())
        .prompt(prompt)
        .build()
        .unwrap();

    let r1 = chain.invoke(prompt_args! { "input" => "a" }).await.unwrap();
    let r2 = chain.invoke(prompt_args! { "input" => "b" }).await.unwrap();
    let r3 = chain.invoke(prompt_args! { "input" => "c" }).await.unwrap();

    assert_eq!(r1, "first");
    assert_eq!(r2, "second");
    assert_eq!(r3, "third");
    assert_eq!(fake.call_count(), 3);
}

// ---------------------------------------------------------------------------
// ConversationalChain — memory persists across turns
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_conversational_chain_memory() {
    let fake = FakeLLM::new(vec!["turn-1 response", "turn-2 response"]);
    let memory = SimpleMemory::new();

    let chain = ConversationalChainBuilder::new()
        .llm(fake.clone())
        .memory(memory.into())
        .build()
        .unwrap();

    let r1 = chain
        .invoke(prompt_args! { "input" => "Hello" })
        .await
        .unwrap();
    let r2 = chain
        .invoke(prompt_args! { "input" => "How are you?" })
        .await
        .unwrap();

    assert_eq!(r1, "turn-1 response");
    assert_eq!(r2, "turn-2 response");
    // Both turns consumed — memory is wired through the chain
    assert_eq!(fake.call_count(), 2);
}

// ---------------------------------------------------------------------------
// SequentialChain — two chains run in series
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_sequential_chain() {
    // Two LLMChains with different prompt variables piped together.
    // SequentialChain feeds the output of chain-1 into chain-2's input_variables.
    let fake1 = FakeLLM::new(vec!["step-one-output"]);
    let fake2 = FakeLLM::new(vec!["step-two-output"]);

    let prompt1 = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{input}", "input")
    ))];
    let prompt2 = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{output}", "output")
    ))];

    let chain1 = LLMChainBuilder::new()
        .llm(fake1.clone())
        .prompt(prompt1)
        .output_key("output")
        .build()
        .unwrap();

    let chain2 = LLMChainBuilder::new()
        .llm(fake2.clone())
        .prompt(prompt2)
        .build()
        .unwrap();

    let seq = SequentialChainBuilder::new()
        .add_chain(chain1)
        .add_chain(chain2)
        .build();

    let result = seq
        .invoke(prompt_args! { "input" => "start" })
        .await
        .unwrap();

    assert_eq!(result, "step-two-output");
    assert_eq!(fake1.call_count(), 1);
    assert_eq!(fake2.call_count(), 1);
}

// ---------------------------------------------------------------------------
// LLMChain::get_input_keys reflects prompt variables
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_llm_chain_input_keys() {
    let fake = FakeLLM::new(vec![]);

    let prompt = message_formatter![fmt_template!(HumanMessagePromptTemplate::new(
        template_fstring!("{question}", "question")
    ))];

    let chain = LLMChainBuilder::new()
        .llm(fake)
        .prompt(prompt)
        .build()
        .unwrap();

    let keys = chain.get_input_keys();
    assert!(
        keys.contains(&"question".to_string()),
        "input keys should contain 'question', got {:?}",
        keys
    );
}

// ---------------------------------------------------------------------------
// Tool trait unification — a custom Tool impl satisfies both core and tools APIs
// ---------------------------------------------------------------------------

/// A trivial tool that echoes its input. Implements langchainx_core::tools::Tool
/// via the re-exported `langchainx::tools::Tool` trait.
struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String {
        "echo".into()
    }

    fn description(&self) -> String {
        "Echoes the input back".into()
    }

    async fn run(&self, input: serde_json::Value) -> Result<String, ToolError> {
        Ok(input.as_str().unwrap_or("no input").to_string())
    }
}

#[tokio::test]
async fn test_tool_trait_unified_across_crates() {
    // Prove that a Tool impl can be wrapped in Arc<dyn Tool> and used via the
    // same trait that langchainx-agent and langchainx-tools expect.
    let tool: Arc<dyn Tool> = Arc::new(EchoTool);

    assert_eq!(tool.name(), "echo");

    let result = tool.call("hello").await.unwrap();
    assert_eq!(result, "hello");

    // Verify ToolError variants are accessible from the same path
    let err = ToolError::InvalidInput("test".into());
    assert!(err.to_string().contains("invalid input"));
}

#[tokio::test]
async fn test_tool_from_langchainx_tools_crate() {
    // CommandExecutor is defined in langchainx-tools and implements
    // langchainx_core::tools::Tool. Verify it can be used as Arc<dyn Tool>
    // through the root re-export — proving no trait mismatch.
    let executor = langchainx::tools::CommandExecutor::new("bash");
    let tool: Arc<dyn Tool> = Arc::new(executor);
    assert_eq!(tool.name(), "Command_Executor");
}
