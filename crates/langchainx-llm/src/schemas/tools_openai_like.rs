//! Provider-neutral function-call types convertible to OpenAI request types.
use crate::schemas::convert::{
    LangchainFromOpenAI, OpenAIFromLangchain, TryLangchainFromOpenAI, TryOpenAiFromLangchain,
};
use async_openai::error::OpenAIError;
use async_openai::types::{
    ChatCompletionNamedToolChoice, ChatCompletionTool, ChatCompletionToolArgs,
    ChatCompletionToolChoiceOption, ChatCompletionToolType, FunctionName, FunctionObjectArgs,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
/// Controls function selection for a model request.
pub enum FunctionCallBehavior {
    /// Prevents the model from calling a function.
    None,
    /// Lets the model decide whether to call a function.
    Auto,
    /// Requires the model to call one or more functions.
    Required,
    /// Requires a call to the function with the supplied name.
    Named(String),
}

impl OpenAIFromLangchain<FunctionCallBehavior> for ChatCompletionToolChoiceOption {
    fn from_langchain(langchain: FunctionCallBehavior) -> Self {
        match langchain {
            FunctionCallBehavior::Auto => ChatCompletionToolChoiceOption::Auto,
            FunctionCallBehavior::None => ChatCompletionToolChoiceOption::None,
            FunctionCallBehavior::Required => ChatCompletionToolChoiceOption::Required,
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

impl LangchainFromOpenAI<ChatCompletionToolChoiceOption> for FunctionCallBehavior {
    fn from_openai(openai: ChatCompletionToolChoiceOption) -> Self {
        match openai {
            ChatCompletionToolChoiceOption::None => Self::None,
            ChatCompletionToolChoiceOption::Auto => Self::Auto,
            ChatCompletionToolChoiceOption::Required => Self::Required,
            ChatCompletionToolChoiceOption::Named(choice) => Self::Named(choice.function.name),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
/// A function exposed to a model for tool calling.
pub struct FunctionDefinition {
    /// Function name sent to the provider.
    pub name: String,
    /// Human-readable description of the function.
    pub description: String,
    /// JSON Schema describing the function arguments.
    pub parameters: Value,
}

impl FunctionDefinition {
    /// Creates a function definition and normalizes spaces in its name to underscores.
    pub fn new(name: &str, description: &str, parameters: Value) -> Self {
        FunctionDefinition {
            name: name.trim().replace(" ", "_"),
            description: description.to_string(),
            parameters,
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

impl TryLangchainFromOpenAI<ChatCompletionTool> for FunctionDefinition {
    type Error = OpenAIError;

    fn try_from_openai(openai: ChatCompletionTool) -> Result<Self, Self::Error> {
        let function = openai.function;
        if function.strict.is_some() {
            return Err(OpenAIError::InvalidArgument(
                "FunctionDefinition cannot represent OpenAI strict mode".to_owned(),
            ));
        }
        let description = function.description.ok_or_else(|| {
            OpenAIError::InvalidArgument(
                "function description is required by FunctionDefinition".to_owned(),
            )
        })?;
        let parameters = function.parameters.ok_or_else(|| {
            OpenAIError::InvalidArgument(
                "function parameters are required by FunctionDefinition".to_owned(),
            )
        })?;

        Ok(Self {
            name: function.name,
            description,
            parameters,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// A function call returned by an OpenAI-compatible model.
pub struct FunctionCallResponse {
    /// Provider-assigned identifier for the tool call.
    pub id: String,
    #[serde(rename = "type")]
    /// Provider type discriminator for the call.
    pub type_field: String,
    /// Function name and serialized arguments.
    pub function: FunctionDetail,
}

#[derive(Serialize, Deserialize, Debug)]
/// The function selected by a model and its serialized arguments.
pub struct FunctionDetail {
    /// Name of the function to invoke.
    pub name: String,
    /// JSON-encoded arguments to deserialize according to the function's schema.
    pub arguments: String,
}

impl FunctionCallResponse {
    /// Parses a function call from its JSON representation.
    pub fn parse(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
