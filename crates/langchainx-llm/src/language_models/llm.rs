//! The provider-independent language-model interface.
use std::{future::Future, pin::Pin, sync::Arc};

use futures::Stream;

use crate::schemas::{Message, StreamData};

use super::{GenerateResult, LLMError, options::CallOptions};

/// A sendable stream of language-model response chunks.
pub type LLMStream = Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>;

/// Defines a language model that can generate and stream responses.
pub trait LLM: Sync + Send {
    /// Generates a complete response for a sequence of chat messages.
    fn generate(
        &self,
        messages: &[Message],
    ) -> impl Future<Output = Result<GenerateResult, LLMError>> + Send;
    /// Generates a response for a single human prompt.
    fn invoke(&self, prompt: &str) -> impl Future<Output = Result<String, LLMError>> + Send {
        async move {
            self.generate(&[Message::new_human_message(prompt)])
                .await
                .map(|res| res.generation)
        }
    }
    /// Streams response chunks for a sequence of chat messages.
    fn stream(
        &self,
        _messages: &[Message],
    ) -> impl Future<Output = Result<LLMStream, LLMError>> + Send;

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

/// A boxed, sendable future used at dynamic language-model boundaries.
pub type BoxLLMFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Object-safe adapter for dynamically dispatched [`LLM`] implementations.
pub trait DynLLM: Sync + Send {
    /// Generates a response through a boxed future.
    fn dyn_generate<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<GenerateResult, LLMError>>;
    /// Invokes the model through a boxed future.
    fn dyn_invoke<'a>(&'a self, prompt: &'a str) -> BoxLLMFuture<'a, Result<String, LLMError>>;
    /// Streams model output through a boxed future.
    fn dyn_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<LLMStream, LLMError>>;
    /// Applies call options to the wrapped model.
    fn dyn_add_options(&mut self, options: CallOptions);
    /// Formats messages using the wrapped model's representation.
    fn dyn_messages_to_string(&self, messages: &[Message]) -> String;
}

impl<L: LLM> DynLLM for L {
    fn dyn_generate<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<GenerateResult, LLMError>> {
        Box::pin(LLM::generate(self, messages))
    }

    fn dyn_invoke<'a>(&'a self, prompt: &'a str) -> BoxLLMFuture<'a, Result<String, LLMError>> {
        Box::pin(LLM::invoke(self, prompt))
    }

    fn dyn_stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<LLMStream, LLMError>> {
        Box::pin(LLM::stream(self, messages))
    }

    fn dyn_add_options(&mut self, options: CallOptions) {
        LLM::add_options(self, options);
    }

    fn dyn_messages_to_string(&self, messages: &[Message]) -> String {
        LLM::messages_to_string(self, messages)
    }
}

/// Conversion helper so builders can accept concrete LLMs and dynamic LLM adapters.
pub trait IntoArcLLM {
    /// Converts this value into a shared dynamic language model.
    fn into_arc_llm(self) -> Arc<dyn DynLLM>;
}

impl<L: LLM + 'static> IntoArcLLM for L {
    fn into_arc_llm(self) -> Arc<dyn DynLLM> {
        Arc::new(self)
    }
}

impl IntoArcLLM for Arc<dyn DynLLM> {
    fn into_arc_llm(self) -> Arc<dyn DynLLM> {
        self
    }
}
