use std::sync::Arc;

use langchainx::{
    chain::{Chain, ChainError},
    language_models::{GenerateResult, LLMError, llm::LLM},
    prompt::PromptArgs,
    schemas::Message,
    tools::{Tool, ToolError},
};
use serde_json::Value;

struct NativeLlm;

impl LLM for NativeLlm {
    async fn generate(&self, _messages: &[Message]) -> Result<GenerateResult, LLMError> {
        Ok(GenerateResult {
            generation: "native llm".to_string(),
            ..Default::default()
        })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<
        std::pin::Pin<
            Box<
                dyn futures::Stream<Item = Result<langchainx::schemas::StreamData, LLMError>>
                    + Send,
            >,
        >,
        LLMError,
    > {
        Ok(Box::pin(futures::stream::empty()))
    }
}

struct NativeChain;

impl Chain for NativeChain {
    async fn call(&self, _input: PromptArgs) -> Result<GenerateResult, ChainError> {
        Ok(GenerateResult {
            generation: "native chain".to_string(),
            ..Default::default()
        })
    }
}

struct NativeTool;

impl Tool for NativeTool {
    fn name(&self) -> String {
        "native_tool".to_string()
    }

    fn description(&self) -> String {
        "Exercises native async trait dispatch".to_string()
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        Ok(input.to_string())
    }
}

#[tokio::test]
async fn hot_path_traits_accept_native_async_implementations() {
    fn assert_send<T: Send>(_: T) {}

    assert_send(NativeLlm.generate(&[]));
    assert_send(NativeChain.call(PromptArgs::new()));
    assert_send(NativeTool.run(Value::Null));

    assert_eq!(NativeLlm.invoke("hello").await.unwrap(), "native llm");
    assert_eq!(
        NativeChain.invoke(PromptArgs::new()).await.unwrap(),
        "native chain"
    );
    assert_eq!(NativeTool.call("hello").await.unwrap(), "\"hello\"");
}

#[tokio::test]
async fn dynamic_boundaries_box_only_object_safe_calls() {
    let llm: Arc<dyn langchainx::language_models::llm::DynLLM> = Arc::new(NativeLlm);
    let chain: Box<dyn langchainx::chain::DynChain> = Box::new(NativeChain);
    let tool: Arc<dyn langchainx::tools::DynTool> = Arc::new(NativeTool);

    assert_eq!(llm.invoke("hello").await.unwrap(), "native llm");
    assert_eq!(
        chain.invoke(PromptArgs::new()).await.unwrap(),
        "native chain"
    );
    assert_eq!(tool.call("hello").await.unwrap(), "\"hello\"");
}
