//! DeepSeek chat-completions implementation of the language-model interface.
use crate::{
    DeepseekError,
    language_models::{GenerateResult, LLMError, TokenUsage, llm::LLM, options::CallOptions},
    schemas::{Message, StreamData},
    sse::SseDecoder,
};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::{fmt, pin::Pin};

use super::models::{ApiResponse, DeepseekMessage, Payload, ResponseFormat};

const PENALTY_RANGE_MIN: f32 = -2.0;
const PENALTY_RANGE_MAX: f32 = 2.0;

/// DeepSeek model identifiers supported by the convenience enum.
pub enum DeepseekModel {
    /// General-purpose DeepSeek chat model.
    DeepseekChat,
    /// DeepSeek reasoning model.
    DeepseekReasoner,
}

impl fmt::Display for DeepseekModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            DeepseekModel::DeepseekChat => "deepseek-chat",
            DeepseekModel::DeepseekReasoner => "deepseek-reasoner",
        };
        write!(f, "{s}")
    }
}

#[derive(Clone)]
/// A client for generating and streaming responses from DeepSeek.
pub struct Deepseek {
    model: String,
    options: CallOptions,
    api_key: String,
    base_url: String,
    json_mode: bool,
    include_reasoning: bool,
}

impl Default for Deepseek {
    fn default() -> Self {
        Self::new()
    }
}

impl Deepseek {
    /// Creates a client using `DEEPSEEK_API_KEY` and the default chat model.
    pub fn new() -> Self {
        Self {
            model: DeepseekModel::DeepseekChat.to_string(),
            options: CallOptions::default(),
            api_key: std::env::var("DEEPSEEK_API_KEY").unwrap_or_default(),
            base_url: "https://api.deepseek.com".to_string(),
            json_mode: false,
            include_reasoning: false,
        }
    }

    /// Sets the DeepSeek model identifier.
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }

    /// Replaces the model call options.
    pub fn with_options(mut self, options: CallOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the DeepSeek API key.
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// Sets the API base URL.
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Enables or disables JSON-object response mode.
    pub fn with_json_mode(mut self, json_mode: bool) -> Self {
        self.json_mode = json_mode;
        self
    }

    /// Controls whether reasoner output includes reasoning content.
    pub fn with_include_reasoning(mut self, include_reasoning: bool) -> Self {
        self.include_reasoning = include_reasoning;
        self
    }

    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        let client = Client::new();
        let payload = self.build_payload(messages, false);
        let res = client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let status = res.status().as_u16();

        let res = match status {
            400 => Err(LLMError::DeepseekError(DeepseekError::InvalidFormatError(
                "Invalid request format".to_string(),
            ))),
            401 => Err(LLMError::DeepseekError(DeepseekError::AuthenticationError(
                "Invalid API Key".to_string(),
            ))),
            402 => Err(LLMError::DeepseekError(
                DeepseekError::InsufficientBalanceError("Insufficient balance".to_string()),
            )),
            422 => Err(LLMError::DeepseekError(
                DeepseekError::InvalidParametersError("Invalid parameters".to_string()),
            )),
            429 => Err(LLMError::DeepseekError(DeepseekError::RateLimitError(
                "Rate limit reached".to_string(),
            ))),
            500 => Err(LLMError::DeepseekError(DeepseekError::ServerError(
                "Server error".to_string(),
            ))),
            503 => Err(LLMError::DeepseekError(
                DeepseekError::ServerOverloadedError("Server overloaded".to_string()),
            )),
            _ => Ok(res.json::<ApiResponse>().await?),
        }?;

        let choice = res.choices.first();

        let mut generation = choice
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        // If include_reasoning is enabled and the model is deepseek-reasoner,
        // append the reasoning content to the generation if available
        if self.include_reasoning
            && self.model == DeepseekModel::DeepseekReasoner.to_string()
            && let Some(reasoning) = choice.and_then(|c| c.message.reasoning_content.clone())
        {
            generation = format!("Reasoning:\n{}\n\nAnswer:\n{}", reasoning, generation);
        }

        let tokens = Some(TokenUsage {
            prompt_tokens: res.usage.prompt_tokens,
            completion_tokens: res.usage.completion_tokens,
            total_tokens: res.usage.total_tokens,
        });

        Ok(GenerateResult { tokens, generation })
    }

    fn build_payload(&self, messages: &[Message], stream: bool) -> Payload {
        let mut response_format = None;
        if self.json_mode {
            response_format = Some(ResponseFormat {
                format_type: "json_object".to_string(),
            });
        }

        let mut payload = Payload {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(DeepseekMessage::from_message)
                .collect::<Vec<_>>(),
            max_tokens: self.options.max_tokens,
            stream: None,
            temperature: self.options.temperature,
            top_p: self.options.top_p,
            frequency_penalty: None,
            presence_penalty: None,
            stop: self.options.stop_words.clone(),
            response_format,
        };

        if stream {
            payload.stream = Some(true);
        }

        // Apply frequency_penalty if it's in the options range
        if let Some(fp) = self.options.frequency_penalty
            && (PENALTY_RANGE_MIN..=PENALTY_RANGE_MAX).contains(&fp)
        {
            payload.frequency_penalty = Some(fp);
        }

        // Apply presence_penalty if it's in the options range
        if let Some(pp) = self.options.presence_penalty
            && (PENALTY_RANGE_MIN..=PENALTY_RANGE_MAX).contains(&pp)
        {
            payload.presence_penalty = Some(pp);
        }

        payload
    }
}

#[async_trait]
impl LLM for Deepseek {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        self.generate(messages).await
    }

    // qual:allow(iosp) reason: "streaming I/O boundary"
    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let client = Client::new();
        let payload = self.build_payload(messages, true);
        let request = client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .build()?;

        let stream = client.execute(request).await?;
        let stream = stream.bytes_stream();

        let include_reasoning = self.include_reasoning;
        let is_reasoner = self.model == DeepseekModel::DeepseekReasoner.to_string();

        let processed_stream = async_stream::try_stream! {
            let mut decoder = SseDecoder::default();
            futures::pin_mut!(stream);
            while let Some(result) = stream.next().await {
                let bytes = result.map_err(|error| LLMError::OtherError(error.to_string()))?;
                for data in decoder.push(&bytes)? {
                    let chunk: Value = serde_json::from_str(&data)?;
                    if let Some(data) = deepseek_stream_data(chunk, include_reasoning, is_reasoner) {
                        yield data;
                    }
                }
            }
            decoder.finish()?;
        };

        Ok(Box::pin(processed_stream))
    }

    fn add_options(&mut self, options: CallOptions) {
        self.options = options;
    }
}

fn deepseek_stream_data(
    chunk: Value,
    include_reasoning: bool,
    is_reasoner: bool,
) -> Option<StreamData> {
    let delta = chunk.get("choices")?.as_array()?.first()?.get("delta")?;
    let usage = chunk.get("usage").map(deepseek_token_usage);
    if include_reasoning
        && is_reasoner
        && let Some(reasoning) = delta
            .get("reasoning_content")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
    {
        return Some(StreamData::new(
            chunk.clone(),
            usage,
            format!("Reasoning: {reasoning}"),
        ));
    }
    delta
        .get("content")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(|content| StreamData::new(chunk.clone(), usage, content))
}

fn deepseek_token_usage(usage: &Value) -> TokenUsage {
    TokenUsage {
        prompt_tokens: usage
            .get("prompt_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        completion_tokens: usage
            .get("completion_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
        total_tokens: usage
            .get("total_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::{Message, MessageType};

    #[tokio::test]
    async fn generate_uses_configured_endpoint_and_parses_usage() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
                    "id":"response-1","object":"chat.completion","created":1,
                    "model":"deepseek-chat","choices":[{"message":{"role":"assistant","content":"pong","name":null,"reasoning_content":null},"finish_reason":"stop","index":0}],
                    "usage":{"prompt_tokens":2,"completion_tokens":1,"total_tokens":3},
                    "system_fingerprint":"test"
                }"#,
            )
            .create_async()
            .await;
        let client = Deepseek::new()
            .with_api_key("test-key")
            .with_base_url(server.url());

        let result = client
            .generate(&[Message::new_human_message("ping")])
            .await
            .unwrap();

        assert_eq!(result.generation, "pong");
        assert_eq!(result.tokens.unwrap().total_tokens, 3);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn generate_maps_rate_limit_status_to_typed_error() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/v1/chat/completions")
            .with_status(429)
            .create_async()
            .await;
        let client = Deepseek::new().with_base_url(server.url());

        let result = client.generate(&[Message::new_human_message("ping")]).await;

        assert!(matches!(
            result,
            Err(LLMError::DeepseekError(DeepseekError::RateLimitError(_)))
        ));
        mock.assert_async().await;
    }

    #[tokio::test]
    #[ignore]
    async fn test_deepseek_generate() {
        let messages = vec![Message {
            content: "Hello".to_string(),
            message_type: MessageType::HumanMessage,
            id: Some("test_id".to_string()),
            images: None,
            tool_calls: None,
        }];

        let client = Deepseek::new();
        let res = client.generate(&messages).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_deepseek_stream() {
        let messages = vec![Message {
            content: "Hello".to_string(),
            message_type: MessageType::HumanMessage,
            id: Some("test_id".to_string()),
            images: None,
            tool_calls: None,
        }];

        let client = Deepseek::new();
        let res = client.stream(&messages).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_deepseek_reasoner() {
        let messages = vec![Message {
            content: "9.11 and 9.8, which is greater?".to_string(),
            message_type: MessageType::HumanMessage,
            id: Some("test_id".to_string()),
            images: None,
            tool_calls: None,
        }];

        // Create a client with the DeepseekReasoner model and enable reasoning content
        let client = Deepseek::new()
            .with_model(DeepseekModel::DeepseekReasoner.to_string())
            .with_include_reasoning(true);

        let res = client.generate(&messages).await;
        assert!(res.is_ok());

        // The response will contain both the reasoning and answer content
        if let Ok(result) = res {
            println!("Generation result: {}", result.generation);
        }
    }
}
