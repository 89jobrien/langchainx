use std::sync::Arc;

use async_openai::Client;
use async_openai::types::CreateSpeechRequestArgs;
pub use async_openai::{
    config::{Config, OpenAIConfig},
    types::{SpeechModel, SpeechResponseFormat, Voice},
};
use async_trait::async_trait;
use serde_json::Value;

use crate::{SpeechStorage, Tool, ToolError};

#[derive(Clone)]
pub struct Text2SpeechOpenAI<C: Config> {
    config: C,
    model: SpeechModel,
    voice: Voice,
    storage: Option<Arc<dyn SpeechStorage>>,
    response_format: SpeechResponseFormat,
    path: String,
}

impl<C: Config> Text2SpeechOpenAI<C> {
    pub fn new(config: C) -> Self {
        Self {
            config,
            model: SpeechModel::Tts1,
            voice: Voice::Alloy,
            storage: None,
            response_format: SpeechResponseFormat::Mp3,
            path: "./data/audio.mp3".to_string(),
        }
    }

    pub fn with_model(mut self, model: SpeechModel) -> Self {
        self.model = model;
        self
    }

    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.voice = voice;
        self
    }

    pub fn with_storage<SS: SpeechStorage + 'static>(mut self, storage: SS) -> Self {
        self.storage = Some(Arc::new(storage));
        self
    }

    pub fn with_response_format(mut self, response_format: SpeechResponseFormat) -> Self {
        self.response_format = response_format;
        self
    }

    pub fn with_path<S: Into<String>>(mut self, path: S) -> Self {
        self.path = path.into();
        self
    }

    pub fn with_config(mut self, config: C) -> Self {
        self.config = config;
        self
    }
}

impl Default for Text2SpeechOpenAI<OpenAIConfig> {
    fn default() -> Self {
        Self::new(OpenAIConfig::default())
    }
}

#[async_trait]
impl<C: Config + Send + Sync> Tool for Text2SpeechOpenAI<C> {
    fn name(&self) -> String {
        "Text2SpeechOpenAI".to_string()
    }

    fn description(&self) -> String {
        r#"A wrapper around OpenAI Text2Speech. "
        "Useful for when you need to convert text to speech. "
        "It supports multiple languages, including English, German, Polish, "
        "Spanish, Italian, French, Portuguese""#
            .to_string()
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let input = input
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("input must be a string".to_string()))?;
        let client = Client::with_config(self.config.clone());
        let response_format: SpeechResponseFormat = self.response_format;

        let request = CreateSpeechRequestArgs::default()
            .input(input)
            .voice(self.voice.clone())
            .response_format(response_format)
            .model(self.model.clone())
            .build()
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        let response = client
            .audio()
            .speech(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        if let Some(storage) = self.storage.as_ref() {
            let data = response.bytes;
            return storage
                .save(&self.path, &data)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()));
        } else {
            response
                .save(&self.path)
                .await
                .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        }

        Ok(self.path.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use async_trait::async_trait;
    use tokio::sync::Mutex;

    use crate::SpeechStorage;
    use crate::{Text2SpeechOpenAI, Tool};

    #[tokio::test]
    #[ignore]
    async fn openai_speech2text_tool() {
        let openai = Text2SpeechOpenAI::default();
        let s = openai.call("Hola como estas").await.unwrap();
        println!("{}", s);
    }

    #[derive(Clone, Default)]
    struct CapturingStorage {
        saved: Arc<Mutex<Vec<(String, Vec<u8>)>>>,
    }

    #[async_trait]
    impl SpeechStorage for CapturingStorage {
        async fn save(&self, key: &str, data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
            self.saved
                .lock()
                .await
                .push((key.to_string(), data.to_vec()));
            Ok(key.to_string())
        }
    }

    #[tokio::test]
    async fn run_uses_configured_openai_client() {
        let mut server = mockito::Server::new_async().await;
        let mock = server
            .mock("POST", "/audio/speech")
            .match_header("authorization", "Bearer test-key")
            .with_status(200)
            .with_header("content-type", "audio/mpeg")
            .with_body("audio-bytes")
            .create_async()
            .await;

        let storage = CapturingStorage::default();
        let saved = storage.saved.clone();
        let config = async_openai::config::OpenAIConfig::default()
            .with_api_base(server.url())
            .with_api_key("test-key");
        let openai = Text2SpeechOpenAI::new(config)
            .with_storage(storage)
            .with_path("speech.mp3");

        let path = openai
            .run(serde_json::Value::String("hello".to_string()))
            .await
            .unwrap();

        assert_eq!(path, "speech.mp3");
        assert_eq!(
            saved.lock().await.as_slice(),
            &[("speech.mp3".to_string(), b"audio-bytes".to_vec())]
        );
        mock.assert_async().await;
    }
}
