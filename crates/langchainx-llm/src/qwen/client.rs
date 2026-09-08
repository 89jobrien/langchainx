//! Qwen OpenAI-compatible chat-completions client.
use crate::{
    language_models::{GenerateResult, LLMError, TokenUsage, llm::LLM, options::CallOptions},
    schemas::{Message, StreamData},
    sse::SseDecoder,
};
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde_json::Value;
use std::pin::Pin;

use super::models::{ApiResponse, ErrorResponse, Payload, QwenMessage};
use super::request::QwenModel;
use super::response::parse_error_response;

/// A client for generating and streaming responses from Qwen.
#[derive(Clone)]
pub struct Qwen {
    model: String,
    options: CallOptions,
    api_key: String,
    base_url: String,
}

impl Default for Qwen {
    fn default() -> Self {
        Self::new()
    }
}

impl Qwen {
    /// Creates a client using `QWEN_API_KEY` and the Qwen Turbo model.
    pub fn new() -> Self {
        Self {
            model: QwenModel::QwenTurbo.to_string(), // Default to Turbo model
            options: CallOptions::default(),
            api_key: std::env::var("QWEN_API_KEY").unwrap_or_default(),
            base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions"
                .to_string(),
        }
    }

    /// Sets the Qwen model identifier.
    pub fn with_model<S: Into<String>>(mut self, model: S) -> Self {
        self.model = model.into();
        self
    }

    /// Replaces the model call options.
    pub fn with_options(mut self, options: CallOptions) -> Self {
        self.options = options;
        self
    }

    /// Sets the Qwen API key.
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// Sets the chat-completions endpoint URL.
    pub fn with_base_url<S: Into<String>>(mut self, base_url: S) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Generates text using the Qwen API
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        let client = Client::new();
        let payload = self.build_payload(messages, false);
        let res = client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        match res.status().as_u16() {
            200 => {
                let api_response = res.json::<ApiResponse>().await?;

                // Extract the first choice content
                let generation = match api_response.choices.first() {
                    Some(choice) => choice.message.content.clone(),
                    None => {
                        return Err(LLMError::ContentNotFound(
                            "No content returned from API".to_string(),
                        ));
                    }
                };

                let tokens = Some(TokenUsage {
                    prompt_tokens: api_response.usage.prompt_tokens,
                    completion_tokens: api_response.usage.completion_tokens,
                    total_tokens: api_response.usage.total_tokens,
                });

                Ok(GenerateResult { tokens, generation })
            }
            400 | 401 | 429 | 500 | 503 => {
                let error = res.json::<ErrorResponse>().await?;
                Err(parse_error_response(error.code.as_str(), &error.message))
            }
            _ => {
                let error = res.json::<ErrorResponse>().await?;
                Err(parse_error_response(error.code.as_str(), &error.message))
            }
        }
    }

    /// Builds the API payload from messages
    fn build_payload(&self, messages: &[Message], stream: bool) -> Payload {
        let mut payload = Payload {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(QwenMessage::from_message)
                .collect::<Vec<_>>(),
            max_tokens: self.options.max_tokens,
            stream: None,
            stop: self.options.stop_words.clone(),
            temperature: self.options.temperature,
            top_p: self.options.top_p,
            seed: None,          // Optional
            result_format: None, // Optional
        };

        if stream {
            payload.stream = Some(true);
        }

        payload
    }
}

impl LLM for Qwen {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError> {
        self.generate(messages).await
    }

    async fn stream(
        &self,
        messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let client = Client::new();
        let payload = self.build_payload(messages, true);
        let request = client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("Accept", "text/event-stream")
            .json(&payload)
            .build()?;

        let stream = client.execute(request).await?;
        let stream = stream.bytes_stream();

        let processed_stream = async_stream::try_stream! {
            let mut decoder = SseDecoder::default();
            futures::pin_mut!(stream);
            while let Some(result) = stream.next().await {
                let bytes = result.map_err(LLMError::RequestError)?;
                for data in decoder.push(&bytes)? {
                    let chunk: Value = serde_json::from_str(&data)?;
                    if let Some(content) = chunk
                        .get("choices")
                        .and_then(Value::as_array)
                        .and_then(|choices| choices.first())
                        .and_then(|choice| choice.get("delta"))
                        .and_then(|delta| delta.get("content"))
                        .and_then(Value::as_str)
                        .filter(|content| !content.is_empty())
                    {
                        let usage = chunk.get("usage").map(token_usage);
                        yield StreamData::new(chunk.clone(), usage, content);
                    }
                }
            }
            decoder.finish()?;
        };

        Ok(Box::pin(processed_stream))
    }

    fn add_options(&mut self, options: CallOptions) {
        self.options.merge_options(options)
    }
}

fn token_usage(usage: &Value) -> TokenUsage {
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
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::test;

    #[tokio::test]
    async fn generate_uses_configured_endpoint() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/chat")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"id":"response-1","created":1,"model":"qwen-turbo","choices":[{"message":{"role":"assistant","content":"pong"},"finish_reason":"stop","index":0}],"usage":{"prompt_tokens":2,"completion_tokens":1,"total_tokens":3}}"#,
            )
            .create_async()
            .await;
        let client = Qwen::new()
            .with_api_key("test-key")
            .with_base_url(format!("{}/chat", server.url()));

        let result = client
            .generate(&[Message::new_human_message("ping")])
            .await
            .unwrap();

        assert_eq!(result.generation, "pong");
        assert_eq!(result.tokens.unwrap().total_tokens, 3);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn stream_reassembles_fragmented_sse_frames() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await.unwrap();
            let body = b"data: {\"choices\":[{\"delta\":{\"content\":\"caf\xc3\xa9\"}}]}\n\ndata: [DONE]\n\n";
            let headers = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n",
                body.len()
            );
            socket.write_all(headers.as_bytes()).await.unwrap();
            let split = body.iter().position(|byte| *byte == 0xc3).unwrap() + 1;
            socket.write_all(&body[..split]).await.unwrap();
            socket.flush().await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            socket.write_all(&body[split..]).await.unwrap();
        });
        let client = Qwen::new()
            .with_api_key("test-key")
            .with_base_url(format!("http://{address}/chat"));

        let stream = LLM::stream(&client, &[Message::new_human_message("ping")])
            .await
            .unwrap();
        let chunks = stream.collect::<Vec<_>>().await;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].as_ref().unwrap().content, "caf\u{e9}");
    }

    #[test]
    #[ignore]
    async fn test_qwen_generate() {
        let qwen = Qwen::new();

        let res = qwen
            .generate(&[Message::new_human_message("Hello!")])
            .await
            .unwrap();

        println!("{:?}", res)
    }

    #[test]
    #[ignore]
    async fn test_qwen_stream() {
        let qwen = Qwen::new();
        let mut stream = qwen
            .stream(&[Message::new_human_message("Hello!")])
            .await
            .unwrap();
        while let Some(data) = stream.next().await {
            match data {
                Ok(value) => value.to_stdout().unwrap(),
                Err(e) => panic!("Error invoking LLMChain: {:?}", e),
            }
        }
    }
}
