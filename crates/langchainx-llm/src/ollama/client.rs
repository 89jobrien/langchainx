//! Native Ollama chat implementation of the language-model interface.
use crate::{
    language_models::{GenerateResult, LLMError, TokenUsage, llm::LLM},
    schemas::{Message, MessageType, StreamData},
};
use async_trait::async_trait;
use futures::Stream;
use ollama_rs::generation::{chat::ChatMessageResponse, images::Image};
pub use ollama_rs::{
    Ollama as OllamaClient,
    error::OllamaError,
    generation::{
        chat::{ChatMessage, MessageRole, request::ChatMessageRequest},
        options::GenerationOptions,
    },
};
use std::pin::Pin;
use std::sync::Arc;
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
/// A language-model client backed by a local Ollama server.
pub struct Ollama {
    pub(crate) client: Arc<OllamaClient>,
    pub(crate) model: String,
    pub(crate) options: Option<GenerationOptions>,
}

/// [llama3.2](https://ollama.com/library/llama3.2) is a 3B parameters, 2.0GB model.
const DEFAULT_MODEL: &str = "llama3.2";

impl Ollama {
    /// Creates a client with an Ollama connection, model, and optional generation settings.
    pub fn new<S: Into<String>>(
        client: Arc<OllamaClient>,
        model: S,
        options: Option<GenerationOptions>,
    ) -> Self {
        Ollama {
            client,
            model: model.into(),
            options,
        }
    }

    /// Sets the Ollama model identifier.
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }

    /// Stores generation options on the client.
    pub fn with_options(mut self, options: GenerationOptions) -> Self {
        self.options = Some(options);
        self
    }

    fn generate_request(&self, messages: &[Message]) -> ChatMessageRequest {
        let mapped_messages = messages.iter().map(chat_message_from_message).collect();
        ChatMessageRequest::new(self.model.clone(), mapped_messages)
    }
}
fn chat_message_from_message(message: &Message) -> ChatMessage {
    let images = match message.images.clone() {
        Some(images) => {
            let images = images
                .iter()
                .map(|image| Image::from_base64(&image.image_url))
                .collect();
            Some(images)
        }
        None => None,
    };
    ChatMessage {
        content: message.content.clone(),
        images,
        role: message_role_from_message_type(&message.message_type),
    }
}

fn message_role_from_message_type(message_type: &MessageType) -> MessageRole {
    match message_type {
        MessageType::AIMessage => MessageRole::Assistant,
        MessageType::ToolMessage => MessageRole::Assistant,
        MessageType::SystemMessage => MessageRole::System,
        MessageType::HumanMessage => MessageRole::User,
    }
}

fn map_stream_response(
    response: Result<ChatMessageResponse, ()>,
) -> Option<Result<StreamData, LLMError>> {
    match response {
        Ok(response) => {
            let content = response.message.as_ref()?.content.clone();
            Some(Ok(StreamData::new(
                serde_json::to_value(response).unwrap_or_default(),
                None,
                content,
            )))
        }
        Err(_) => Some(Err(OllamaError::from("Stream error".to_string()).into())),
    }
}

impl Default for Ollama {
    fn default() -> Self {
        let client = Arc::new(OllamaClient::default());
        Ollama::new(client, String::from(DEFAULT_MODEL), None)
    }
}

#[async_trait]
impl LLM for Ollama {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        let request = self.generate_request(messages);
        let result = self.client.send_chat_messages(request).await?;

        let generation = match result.message {
            Some(message) => message.content,
            None => return Err(OllamaError::from("No message in response".to_string()).into()),
        };

        let tokens = result.final_data.map(|final_data| {
            let prompt_tokens = final_data.prompt_eval_count as u32;
            let completion_tokens = final_data.eval_count as u32;
            TokenUsage {
                prompt_tokens,
                completion_tokens,
                total_tokens: prompt_tokens + completion_tokens,
            }
        });

        Ok(GenerateResult { tokens, generation })
    }

    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let request = self.generate_request(messages);
        let result = self.client.send_chat_messages_stream(request).await?;

        let stream = result.filter_map(map_stream_response);

        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ollama_rs::generation::chat::{ChatMessageFinalResponseData, ChatMessageResponse};
    use tokio::io::AsyncWriteExt;
    use tokio_stream::StreamExt;

    fn response(message: Option<ChatMessage>, done: bool) -> ChatMessageResponse {
        ChatMessageResponse {
            model: "llama3.2".to_string(),
            created_at: "2026-09-04T00:00:00Z".to_string(),
            message,
            done,
            final_data: done.then_some(ChatMessageFinalResponseData {
                total_duration: 10,
                prompt_eval_count: 2,
                prompt_eval_duration: 3,
                eval_count: 4,
                eval_duration: 5,
            }),
        }
    }

    #[test]
    fn maps_content_response_to_stream_data() {
        let response = response(Some(ChatMessage::assistant("hello".to_string())), false);

        let mapped = map_stream_response(Ok(response)).unwrap().unwrap();

        assert_eq!(mapped.content, "hello");
        assert_eq!(mapped.value["model"], "llama3.2");
        assert!(mapped.tokens.is_none());
    }

    #[test]
    fn skips_successful_metadata_only_response() {
        assert!(map_stream_response(Ok(response(None, true))).is_none());
    }

    #[test]
    fn maps_transport_error_to_llm_error() {
        let mapped = map_stream_response(Err(())).unwrap();

        assert!(matches!(mapped, Err(LLMError::OllamaError(_))));
    }

    #[tokio::test]
    #[ignore]
    async fn test_generate() {
        let ollama = Ollama::default().with_model("llama3.2");
        let response = ollama.invoke("Hey Macarena, ay").await.unwrap();
        println!("{}", response);
    }

    #[tokio::test]
    #[ignore]
    async fn test_stream() {
        let ollama = Ollama::default().with_model("llama3.2");

        let message = Message::new_human_message("Why does water boil at 100 degrees?");
        let mut stream = ollama.stream(&vec![message]).await.unwrap();
        let mut stdout = tokio::io::stdout();
        while let Some(res) = stream.next().await {
            let data = res.unwrap();
            stdout.write(data.content.as_bytes()).await.unwrap();
        }
        stdout.write(b"\n").await.unwrap();
        stdout.flush().await.unwrap();
    }
}
