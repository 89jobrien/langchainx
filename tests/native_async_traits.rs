use std::sync::Arc;

use futures::StreamExt;
use langchainx::{
    chain::{Chain, ChainError, DynChain},
    language_models::{
        GenerateResult, LLMError,
        llm::{DynLLM, LLM},
    },
    prompt::PromptArgs,
    schemas::{Message, StreamData},
    tools::{DynTool, Tool, ToolError},
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
        Ok(Box::pin(futures::stream::once(async {
            Ok(StreamData::new(Value::Null, None, "llm stream"))
        })))
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

    async fn stream(
        &self,
        _input: PromptArgs,
    ) -> Result<langchainx::chain::ChainStream, ChainError> {
        Ok(Box::pin(futures::stream::once(async {
            Ok(StreamData::new(Value::Null, None, "chain stream"))
        })))
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
    let mut llm: Box<dyn DynLLM> = Box::new(NativeLlm);
    let chain: Box<dyn DynChain> = Box::new(NativeChain);
    let tool: Arc<dyn DynTool> = Arc::new(NativeTool);

    llm.dyn_add_options(Default::default());
    assert_eq!(
        llm.dyn_messages_to_string(&[Message::new_human_message("hello")]),
        "HumanMessage: hello"
    );
    assert_eq!(
        llm.dyn_generate(&[]).await.unwrap().generation,
        "native llm"
    );
    let mut llm_stream = llm.dyn_stream(&[]).await.unwrap();
    assert_eq!(
        llm_stream.next().await.unwrap().unwrap().content,
        "llm stream"
    );
    assert_eq!(llm.dyn_invoke("hello").await.unwrap(), "native llm");

    assert_eq!(
        chain.dyn_call(PromptArgs::new()).await.unwrap().generation,
        "native chain"
    );
    let output = chain.dyn_execute(PromptArgs::new()).await.unwrap();
    assert_eq!(output["output"], Value::String("native chain".to_string()));
    let mut chain_stream = chain.dyn_stream(PromptArgs::new()).await.unwrap();
    assert_eq!(
        chain_stream.next().await.unwrap().unwrap().content,
        "chain stream"
    );
    assert_eq!(
        chain.dyn_invoke(PromptArgs::new()).await.unwrap(),
        "native chain"
    );
    assert!(chain.dyn_required_keys().is_empty());
    assert!(chain.dyn_validate_input(&PromptArgs::new()).is_ok());
    assert!(chain.dyn_get_input_keys().is_empty());
    assert_eq!(chain.dyn_get_output_keys()[0], "output");

    assert_eq!(tool.dyn_name(), "native_tool");
    assert_eq!(
        tool.dyn_description(),
        "Exercises native async trait dispatch"
    );
    assert_eq!(tool.dyn_parameters()["type"], "object");
    assert_eq!(
        tool.dyn_run(Value::String("direct".to_string()))
            .await
            .unwrap(),
        "\"direct\""
    );
    assert_eq!(
        tool.dyn_parse_input(r#"{"input":"parsed"}"#).await,
        Value::String("parsed".to_string())
    );
    assert_eq!(tool.dyn_call("hello").await.unwrap(), "\"hello\"");
}
