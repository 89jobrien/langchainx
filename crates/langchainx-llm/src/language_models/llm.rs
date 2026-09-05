//! The provider-independent language-model interface.
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;

use crate::schemas::{Message, StreamData};

use super::{GenerateResult, LLMError, options::CallOptions};

#[async_trait]
/// Defines a language model that can generate and stream responses.
pub trait LLM: Sync + Send {
    /// Generates a complete response for a sequence of chat messages.
    async fn generate(&self, messages: &[Message]) -> Result<GenerateResult, LLMError>;
    /// Generates a response for a single human prompt.
    async fn invoke(&self, prompt: &str) -> Result<String, LLMError> {
        self.generate(&[Message::new_human_message(prompt)])
            .await
            .map(|res| res.generation)
    }
    /// Streams response chunks for a sequence of chat messages.
    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError>;

    /// Applies call options, allowing chain builders to override model defaults.
    fn add_options(&mut self, _options: CallOptions) {
        // No action taken
    }
    //This is usefull when using non chat models
    /// Formats chat messages as role-prefixed lines for non-chat models.
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
    /// Converts this value into a shared dynamic language model.
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
