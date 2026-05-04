use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::Stream;

use crate::language_models::llm::LLM;
use crate::language_models::{GenerateResult, LLMError};
use crate::schemas::{Message, StreamData};

/// A deterministic LLM test double. Responses are popped from a queue in order.
/// When the queue is exhausted, returns an empty string.
///
/// ```rust
/// use langchainx::test_utils::FakeLLM;
/// use langchainx::language_models::llm::LLM;
///
/// # #[tokio::main]
/// # async fn main() {
/// let llm = FakeLLM::new(vec!["hello".into(), "world".into()]);
/// assert_eq!(llm.invoke("anything").await.unwrap(), "hello");
/// assert_eq!(llm.invoke("anything").await.unwrap(), "world");
/// assert_eq!(llm.call_count(), 2);
/// # }
/// ```
#[derive(Clone)]
pub struct FakeLLM {
    responses: Arc<Mutex<VecDeque<String>>>,
    pub call_count: Arc<AtomicUsize>,
}

impl FakeLLM {
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::from(responses))),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Number of times `generate` has been called.
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    /// Remaining queued responses.
    pub fn remaining(&self) -> usize {
        self.responses.lock().unwrap().len()
    }
}

#[async_trait]
impl LLM for FakeLLM {
    async fn generate(&self, _messages: &[Message]) -> Result<GenerateResult, LLMError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let generation = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default();
        Ok(GenerateResult {
            generation,
            tokens: None,
        })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        Err(LLMError::OtherError(
            "FakeLLM does not support streaming; use generate()".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language_models::llm::LLM;

    #[tokio::test]
    async fn fake_llm_returns_responses_in_order() {
        let llm = FakeLLM::new(vec!["first".into(), "second".into()]);
        assert_eq!(llm.invoke("a").await.unwrap(), "first");
        assert_eq!(llm.invoke("b").await.unwrap(), "second");
        assert_eq!(llm.call_count(), 2);
    }

    #[tokio::test]
    async fn fake_llm_returns_empty_when_exhausted() {
        let llm = FakeLLM::new(vec![]);
        assert_eq!(llm.invoke("x").await.unwrap(), "");
        assert_eq!(llm.call_count(), 1);
    }

    #[tokio::test]
    async fn fake_llm_clone_shares_state() {
        let llm = FakeLLM::new(vec!["shared".into()]);
        let cloned = llm.clone();
        assert_eq!(cloned.invoke("x").await.unwrap(), "shared");
        assert_eq!(llm.call_count(), 1);
        assert_eq!(llm.remaining(), 0);
    }

    #[tokio::test]
    async fn fake_llm_stream_returns_error() {
        let llm = FakeLLM::new(vec![]);
        let msgs = vec![Message::new_human_message("hi")];
        assert!(llm.stream(&msgs).await.is_err());
    }
}
