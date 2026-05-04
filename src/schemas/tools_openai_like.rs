// Struct types re-exported from langchainx-llm for backward compat.
pub use langchainx_llm::schemas::{
    FunctionCallBehavior, FunctionCallResponse, FunctionDefinition, FunctionDetail,
};

// Keep conversion impls using the root crate's convert traits.
use crate::schemas::convert::{OpenAIFromLangchain, TryOpenAiFromLangchain};
use async_openai::types::{
    ChatCompletionNamedToolChoice, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolChoiceOption, ChatCompletionToolType, FunctionName, FunctionObjectArgs,
};

impl OpenAIFromLangchain<FunctionCallBehavior> for ChatCompletionToolChoiceOption {
    fn from_langchain(langchain: FunctionCallBehavior) -> Self {
        match langchain {
            FunctionCallBehavior::Auto => ChatCompletionToolChoiceOption::Auto,
            FunctionCallBehavior::None => ChatCompletionToolChoiceOption::None,
            FunctionCallBehavior::Named(name) => {
                ChatCompletionToolChoiceOption::Named(ChatCompletionNamedToolChoice {
                    r#type: ChatCompletionToolType::Function,
                    function: FunctionName {
                        name: name.to_owned(),
                    },
                })
            }
        }
    }
}

impl TryOpenAiFromLangchain<FunctionDefinition> for ChatCompletionTool {
    type Error = async_openai::error::OpenAIError;
    fn try_from_langchain(langchain: FunctionDefinition) -> Result<Self, Self::Error> {
        let tool = FunctionObjectArgs::default()
            .name(langchain.name)
            .description(langchain.description)
            .parameters(langchain.parameters)
            .build()?;

        ChatCompletionToolArgs::default()
            .r#type(ChatCompletionToolType::Function)
            .function(tool)
            .build()
    }
}
