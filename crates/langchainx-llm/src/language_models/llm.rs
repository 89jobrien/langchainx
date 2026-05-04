use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;

use crate::schemas::{Message, StreamData};

use super::{options::CallOptions, GenerateResult, LLMError};

#[async_trait]
pub trait LLM: Sync + Send {
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError>;
    async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
        self.generate(&[Message::new_human_message(prompt)])
            .await
            .map(|res| res.generation)
    }
    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError>;

    /// This is usefull when you want to create a chain and override
    /// LLM options
    fn add_options(&mut self, _options: CallOptions) {
        // No action taken
    }
    //This is usefull when using non chat models
    fn messages_to_string(&self, messages: &[Message]) -> String {
        messages
            .iter()
            .map(|m| format!("{:?}: {}", m.message_type, m.content))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

/// Conversion helper so builders can accept both concrete LLM types and `Arc<dyn LLM>`.
pub trait IntoArcLLM {
    fn into_arc_llm(self) -> Arc<dyn LLM>;
}

impl<L: LLM + 'static> IntoArcLLM for L {
    fn into_arc_llm(self) -> Arc<dyn LLM> {
        Arc::new(self)
    }
}

impl IntoArcLLM for Arc<dyn LLM> {
    fn into_arc_llm(self) -> Arc<dyn LLM> {
        self
    }
}
