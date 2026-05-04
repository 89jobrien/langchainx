use std::{fmt, pin::Pin};

pub use async_openai::config::{AzureConfig, Config, OpenAIConfig};

use async_openai::types::{ChatCompletionToolChoiceOption, ResponseFormat};
use async_openai::{
    Client,
    error::OpenAIError,
    types::{
        ChatCompletionMessageToolCall, ChatCompletionRequestAssistantMessageArgs,
        ChatCompletionRequestMessage, ChatCompletionRequestMessageContentPartImageArgs,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestToolMessageArgs,
        ChatCompletionRequestUserMessageArgs, ChatCompletionRequestUserMessageContent,
        ChatCompletionRequestUserMessageContentPart, ChatCompletionStreamOptions,
        CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    },
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};

use crate::schemas::convert::{LangchainIntoOpenAI, TryLangchainIntoOpenAI};
use crate::{
    language_models::{GenerateResult, LLMError, TokenUsage, llm::LLM, options::CallOptions},
    schemas::{
        StreamData,
        messages::{Message, MessageType},
    },
};

#[derive(Clone)]
pub enum OpenAIModel {
    Gpt35,
    Gpt4,
    Gpt4Turbo,
    Gpt4o,
    Gpt4oMini,
}

impl fmt::Display for OpenAIModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            OpenAIModel::Gpt35 => "gpt-3.5-turbo",
            OpenAIModel::Gpt4 => "gpt-4",
            OpenAIModel::Gpt4Turbo => "gpt-4-turbo-preview",
            OpenAIModel::Gpt4o => "gpt-4o",
            OpenAIModel::Gpt4oMini => "gpt-4o-mini",
        };
        write!(f, "{s}")
    }
}

impl From<OpenAIModel> for String {
    fn from(val: OpenAIModel) -> Self {
        val.to_string()
    }
}

#[derive(Clone)]
pub struct OpenAI<C: Config> {
    config: C,
    options: CallOptions,
    model: String,
}

impl<C: Config> OpenAI<C> {
    pub fn new(config: C) -> Self {
        Self {
            config,
            options: CallOptions::default(),
            model: OpenAIModel::Gpt4oMini.to_string(),
        }
    }

    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_config(mut self, config: C) -> Self {
        self.config = config;
        self
    }

    pub fn with_options(mut self, options: CallOptions) -> Self {
        self.options = options;
        self
    }
}

impl Default for OpenAI<OpenAIConfig> {
    fn default() -> Self {
        Self::new(OpenAIConfig::default())
    }
}

#[async_trait]
impl<C: Config + Send + Sync + 'static> LLM for OpenAI<C> {
    async fn generate(&self, prompt: &[Message]) -> Result<GenerateResult, LLMError> {
        let client = Client::with_config(self.config.clone());
        let request = self.generate_request(prompt)?;
        let response = client.chat().create(request).await?;
        let mut generate_result = GenerateResult::default();

        if let Some(usage) = response.usage {
            generate_result.tokens = Some(TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            });
        }

        if let Some(choice) = &response.choices.first() {
            generate_result.generation = choice.message.content.clone().unwrap_or_default();
            if let Some(function) = &choice.message.tool_calls {
                generate_result.generation = serde_json::to_string(&function).unwrap_or_default();
            }
        } else {
            generate_result.generation = "".to_string();
        }

        Ok(generate_result)
    }

    async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
        self.generate(&[Message::new_human_message(prompt)])
            .await
            .map(|res| res.generation)
    }

    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let client = Client::with_config(self.config.clone());
        let mut request = self.generate_request(messages)?;
        if let Some(include_usage) = self.options.stream_usage {
            request.stream_options = Some(ChatCompletionStreamOptions { include_usage });
        }

        let original_stream = client.chat().create_stream(request).await?;

        let new_stream = original_stream.map(|result| match result {
            Ok(completion) => {
                let value_completion = serde_json::to_value(completion).map_err(LLMError::from)?;
                if let Some(usage) = value_completion.pointer("/usage")
                    && !usage.is_null()
                {
                    let usage = serde_json::from_value::<TokenUsage>(usage.clone())
                        .map_err(LLMError::from)?;
                    return Ok(StreamData::new(value_completion, Some(usage), ""));
                }
                let content = value_completion
                    .pointer("/choices/0/delta/content")
                    .ok_or(LLMError::ContentNotFound(
                        "/choices/0/delta/content".to_string(),
                    ))?
                    .clone();

                Ok(StreamData::new(
                    value_completion,
                    None,
                    content.as_str().unwrap_or(""),
                ))
            }
            Err(e) => Err(LLMError::from(e)),
        });

        Ok(Box::pin(new_stream))
    }

    fn add_options(&mut self, options: CallOptions) {
        self.options.merge_options(options)
    }
}

impl<C: Config> OpenAI<C> {
    fn to_openai_messages(
        &self,
        messages: &[Message],
    ) -> Result<Vec<ChatCompletionRequestMessage>, LLMError> {
        let mut openai_messages: Vec<ChatCompletionRequestMessage> = Vec::new();
        for m in messages {
            match m.message_type {
                MessageType::AIMessage => openai_messages.push(match &m.tool_calls {
                    Some(value) => {
                        let function: Vec<ChatCompletionMessageToolCall> =
                            serde_json::from_value(value.clone())?;
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .tool_calls(function)
                            .content(m.content.clone())
                            .build()?
                            .into()
                    }
                    None => ChatCompletionRequestAssistantMessageArgs::default()
                        .content(m.content.clone())
                        .build()?
                        .into(),
                }),
                MessageType::HumanMessage => {
                    let content: ChatCompletionRequestUserMessageContent = match m.images.clone() {
                        Some(images) => {
                            let content: Result<
                                Vec<ChatCompletionRequestUserMessageContentPart>,
                                OpenAIError,
                            > = images
                                .into_iter()
                                .map(|image| {
                                    Ok(ChatCompletionRequestMessageContentPartImageArgs::default()
                                        .image_url(image.image_url)
                                        .build()?
                                        .into())
                                })
                                .collect();

                            content?.into()
                        }
                        None => m.content.clone().into(),
                    };

                    openai_messages.push(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(content)
                            .build()?
                            .into(),
                    )
                }
                MessageType::SystemMessage => openai_messages.push(
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(m.content.clone())
                        .build()?
                        .into(),
                ),
                MessageType::ToolMessage => {
                    openai_messages.push(
                        ChatCompletionRequestToolMessageArgs::default()
                            .content(m.content.clone())
                            .tool_call_id(m.id.clone().unwrap_or_default())
                            .build()?
                            .into(),
                    );
                }
            }
        }
        Ok(openai_messages)
    }

    fn generate_request(
        &self,
        messages: &[Message],
    ) -> Result<CreateChatCompletionRequest, LLMError> {
        let messages: Vec<ChatCompletionRequestMessage> = self.to_openai_messages(messages)?;
        let mut request_builder = CreateChatCompletionRequestArgs::default();
        if let Some(temperature) = self.options.temperature {
            request_builder.temperature(temperature);
        }
        if let Some(max_tokens) = self.options.max_tokens {
            request_builder.max_tokens(max_tokens);
        }
        request_builder.model(self.model.to_string());
        if let Some(stop_words) = &self.options.stop_words {
            request_builder.stop(stop_words);
        }

        if let Some(functions) = &self.options.functions {
            let functions: Result<Vec<_>, OpenAIError> = functions
                .clone()
                .into_iter()
                .map(|f| f.try_into_openai())
                .collect();
            request_builder.tools(functions?);
        }

        if let Some(behavior) = &self.options.function_call_behavior {
            request_builder
                .tool_choice::<ChatCompletionToolChoiceOption>(behavior.clone().into_openai());
        }

        if let Some(response_format) = &self.options.response_format {
            request_builder
                .response_format::<ResponseFormat>(response_format.clone().into_openai());
        }

        request_builder.messages(messages);
        Ok(request_builder.build()?)
    }
}
#[cfg(test)]
mod tests {
    use crate::schemas::FunctionDefinition;

    use super::*;

    use base64::prelude::*;
    use serde_json::json;
    use tokio::test;

    #[test]
    #[ignore]
    async fn test_invoke() {
        let open_ai =
            OpenAI::new(OpenAIConfig::default()).with_model(OpenAIModel::Gpt35.to_string());
        match open_ai.invoke("hola").await {
            Ok(result) => println!("Generate Result: {:?}", result),
            Err(e) => eprintln!("Error calling generate: {:?}", e),
        }
    }

    #[test]
    #[ignore]
    async fn test_generate_function() {
        let open_ai =
            OpenAI::new(OpenAIConfig::default()).with_model(OpenAIModel::Gpt35.to_string());
        let messages = vec![Message::new_human_message("Hello, how are you?")];
        match open_ai.generate(&messages).await {
            Ok(result) => println!("Generate Result: {:?}", result),
            Err(e) => eprintln!("Error calling generate: {:?}", e),
        }
    }

    #[test]
    #[ignore]
    async fn test_openai_stream() {
        // Setup the OpenAI client with the necessary options
        let open_ai = OpenAI::default().with_model(OpenAIModel::Gpt35.to_string());

        // Define a set of messages to send to the generate function
        let messages = vec![Message::new_human_message("Hello, how are you?")];

        open_ai
            .stream(&messages)
            .await
            .unwrap()
            .for_each(|result| async {
                match result {
                    Ok(stream_data) => {
                        println!("Stream Data: {:?}", stream_data.content);
                    }
                    Err(e) => {
                        eprintln!("Error calling generate: {:?}", e);
                    }
                }
            })
            .await;
    }

    #[test]
    #[ignore]
    async fn test_function() {
        let mut functions = Vec::new();
        functions.push(FunctionDefinition {
            name: "cli".to_string(),
            description: "Use the Ubuntu command line to preform any action you wish.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The raw command you want executed"
                    }
                },
                "required": ["command"]
            }),
        });

        let llm = OpenAI::default()
            .with_model(OpenAIModel::Gpt35)
            .with_config(OpenAIConfig::new())
            .with_options(CallOptions::new().with_functions(functions));
        let response = llm
            .invoke("Use the command line to create a new rust project. Execute the first command.")
            .await
            .unwrap();
        println!("{}", response)
    }

    #[test]
    #[ignore]
    async fn test_generate_with_image_message() {
        // Setup the OpenAI client with the necessary options
        let open_ai =
            OpenAI::new(OpenAIConfig::default()).with_model(OpenAIModel::Gpt4o.to_string());

        // Convert image to base64
        let image = std::fs::read("./src/llm/test_data/example.jpg").unwrap();
        let image_base64 = BASE64_STANDARD.encode(image);

        // Define a set of messages to send to the generate function
        let image_urls = vec![format!("data:image/jpeg;base64,{image_base64}")];
        let messages = vec![
            Message::new_human_message("Describe this image"),
            Message::new_human_message_with_images(image_urls),
        ];

        // Call the generate function
        let response = open_ai.generate(&messages).await.unwrap();
        println!("Response: {:?}", response);
    }
}
