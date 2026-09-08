use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use futures::{Stream, stream};
use langchainx_core::schemas::{Message, StreamData};
use langchainx_llm::language_models::{GenerateResult, LLMError, llm::LLM};
use tokio::sync::Mutex;

/// A deterministic language model that returns queued responses in order.
#[derive(Clone, Debug)]
pub struct FakeLLM {
    responses: Arc<Mutex<VecDeque<String>>>,
    call_count: Arc<AtomicUsize>,
}

impl FakeLLM {
    /// Creates a fake with responses consumed by successive generate or stream calls.
    pub fn new<I, S>(responses: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().map(Into::into).collect())),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Returns the number of generate and stream calls made so far.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    async fn next_response(&self) -> String {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        self.responses.lock().await.pop_front().unwrap_or_default()
    }
}

impl LLM for FakeLLM {
    async fn generate(&self, _messages: &[Message]) -> Result<GenerateResult, LLMError> {
        Ok(GenerateResult {
            generation: self.next_response().await,
            ..Default::default()
        })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        let response = self.next_response().await;
        let chunk = StreamData::new(serde_json::json!(response), None, response);
        Ok(Box::pin(stream::iter([Ok(chunk)])))
    }
}
