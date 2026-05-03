use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use async_trait::async_trait;
use futures::Stream;
use tokio::sync::Mutex;

use langchain_rust::{
    embedding::{embedder_trait::Embedder, EmbedderError},
    language_models::{llm::LLM, GenerateResult, LLMError},
    schemas::{Message, StreamData},
};

// ---------------------------------------------------------------------------
// FakeLLM
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct FakeLLM {
    pub responses: Arc<Mutex<VecDeque<String>>>,
    pub call_count: Arc<AtomicUsize>,
}

impl FakeLLM {
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(
                responses.into_iter().map(String::from).collect(),
            )),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl LLM for FakeLLM {
    async fn generate(&self, _messages: &[Message]) -> Result<GenerateResult, LLMError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let mut responses = self.responses.lock().await;
        let generation = responses.pop_front().unwrap_or_default();
        Ok(GenerateResult {
            generation,
            ..Default::default()
        })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        unimplemented!("FakeLLM::stream — use FakeStreamingLLM for stream tests")
    }
}

// ---------------------------------------------------------------------------
// FakeEmbedder — returns deterministic unit vectors based on string hash
// ---------------------------------------------------------------------------

pub struct FakeEmbedder {
    pub dims: usize,
}

impl FakeEmbedder {
    pub fn new(dims: usize) -> Self {
        Self { dims }
    }

    fn embed_text(&self, text: &str) -> Vec<f64> {
        // Deterministic: spread a simple hash across `dims` dimensions then
        // normalise so cosine similarity is meaningful.
        let hash = text.bytes().enumerate().fold(0u64, |acc, (i, b)| {
            acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 1))
        });
        let mut v: Vec<f64> = (0..self.dims)
            .map(|i| {
                let x = hash.wrapping_add(i as u64) as f64;
                (x % 100.0) / 100.0 - 0.5
            })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt().max(1e-9);
        v.iter_mut().for_each(|x| *x /= norm);
        v
    }
}

#[async_trait]
impl Embedder for FakeEmbedder {
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError> {
        Ok(documents.iter().map(|d| self.embed_text(d)).collect())
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError> {
        Ok(self.embed_text(text))
    }
}

// ---------------------------------------------------------------------------
// Ollama availability check — skip tests when Ollama is not running
// ---------------------------------------------------------------------------

/// Returns `true` if Ollama is reachable and `model` is available locally.
/// Call at the top of any Ollama-tier test: `if !ollama_available("qwen2.5:0.5b").await { return; }`.
pub async fn ollama_available(model: &str) -> bool {
    let Ok(output) = tokio::process::Command::new("ollama")
        .args(["list"])
        .output()
        .await
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.contains(model)
}
