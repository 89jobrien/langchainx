use std::{future::Future, pin::Pin, sync::Arc};

use futures::Stream;

use crate::schemas::{Message, StreamData};

use super::{GenerateResult, LLMError, options::CallOptions};

pub type LLMStream = Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>;

pub trait LLM: Sync + Send {
    fn generate(
        &self,
        messages: &[Message],
    ) -> impl Future<Output = Result<GenerateResult, LLMError>> + Send;
    fn invoke(&self, prompt: &str) -> impl Future<Output = Result<String, LLMError>> + Send {
        async move {
            self.generate(&[Message::new_human_message(prompt)])
                .await
                .map(|res| res.generation)
        }
    }
    fn stream(
        &self,
        _messages: &[Message],
    ) -> impl Future<Output = Result<LLMStream, LLMError>> + Send;

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

pub type BoxLLMFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DynLLM: Sync + Send {
    fn generate<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<GenerateResult, LLMError>>;
    fn invoke<'a>(&'a self, prompt: &'a str) -> BoxLLMFuture<'a, Result<String, LLMError>>;
    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<LLMStream, LLMError>>;
    fn add_options(&mut self, options: CallOptions);
    fn messages_to_string(&self, messages: &[Message]) -> String;
}

impl<L: LLM> DynLLM for L {
    fn generate<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<GenerateResult, LLMError>> {
        Box::pin(LLM::generate(self, messages))
    }

    fn invoke<'a>(&'a self, prompt: &'a str) -> BoxLLMFuture<'a, Result<String, LLMError>> {
        Box::pin(LLM::invoke(self, prompt))
    }

    fn stream<'a>(
        &'a self,
        messages: &'a [Message],
    ) -> BoxLLMFuture<'a, Result<LLMStream, LLMError>> {
        Box::pin(LLM::stream(self, messages))
    }

    fn add_options(&mut self, options: CallOptions) {
        LLM::add_options(self, options);
    }

    fn messages_to_string(&self, messages: &[Message]) -> String {
        LLM::messages_to_string(self, messages)
    }
}

/// Conversion helper so builders can accept concrete LLMs and dynamic LLM adapters.
pub trait IntoArcLLM {
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
