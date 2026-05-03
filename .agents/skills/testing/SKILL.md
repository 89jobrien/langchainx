---
name: testing
description: JOB-257 — FakeLLM, in-process test doubles, writing offline chain/agent tests.
---

<oneliner>
Every chain test is currently #[ignore] because there is no FakeLLM. Use FakeLLM from
src/test_utils to write offline tests. Never call real APIs in unit tests.
</oneliner>

<current-state>
## Current State (JOB-257)

Every chain and agent test is gated by `#[ignore]`:

```rust
#[tokio::test]
#[ignore]  // requires live OPENAI_API_KEY
async fn test_invoke_chain() { ... }
```

There is zero automated test coverage for chain or agent behavior.
</current-state>

<fakellm>
## FakeLLM (implement in src/test_utils.rs)

```rust
// src/test_utils.rs — gated behind #[cfg(any(test, feature = "test-utils"))]
use std::collections::VecDeque;
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tokio::sync::Mutex;
use async_trait::async_trait;
use langchain_rust::{
    language_models::{llm::LLM, options::CallOptions, GenerateResult, LLMError},
    schemas::{Message, StreamData},
};

#[derive(Clone)]
pub struct FakeLLM {
    pub responses: Arc<Mutex<VecDeque<String>>>,
    pub call_count: Arc<AtomicUsize>,
}

impl FakeLLM {
    pub fn new(responses: Vec<&str>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(
                responses.into_iter().map(String::from).collect()
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
        Ok(GenerateResult { generation, ..Default::default() })
    }

    async fn stream(
        &self,
        _messages: &[Message],
    ) -> Result<std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamData, LLMError>> + Send>>, LLMError> {
        unimplemented!("FakeLLM::stream — use FakeStreamingLLM for stream tests")
    }
}
```

</fakellm>

<writing-tests>
## Writing Offline Chain Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::FakeLLM;
    use langchain_rust::{
        chain::{Chain, LLMChainBuilder},
        message_formatter, fmt_template,
        prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
        prompt_args, template_fstring,
    };

    #[tokio::test]
    async fn test_llm_chain_invoke() {
        let fake = FakeLLM::new(vec!["Hello from FakeLLM!"]);

        let prompt = message_formatter![
            fmt_template!(HumanMessagePromptTemplate::new(
                template_fstring!("{input}", "input")
            )),
        ];

        let chain = LLMChainBuilder::new()
            .llm(fake.clone())
            .prompt(prompt)
            .build()
            .unwrap();

        let result = chain.invoke(prompt_args! { "input" => "Hi" }).await.unwrap();
        assert_eq!(result, "Hello from FakeLLM!");
        assert_eq!(fake.call_count(), 1);
    }

    #[tokio::test]
    async fn test_chain_returns_error_on_empty_responses() {
        let fake = FakeLLM::new(vec![]); // no responses queued
        let chain = LLMChainBuilder::new()
            .llm(fake)
            .prompt(prompt)
            .build()
            .unwrap();

        let result = chain.invoke(prompt_args! { "input" => "Hi" }).await.unwrap();
        assert_eq!(result, ""); // unwrap_or_default in FakeLLM
    }
}
```

</writing-tests>

<local-llm>
## Local LLMs for Tests Needing Real Generated Output

When a test requires actual language model reasoning (not canned responses), use a small
local model via Ollama instead of a cloud API. This keeps tests offline, fast, and free.

### Setup

```bash
# Install Ollama: https://ollama.com
ollama pull qwen2.5:0.5b    # 400MB — fastest, good for basic reasoning
ollama pull llama3.2:1b     # 1.3GB — better quality
ollama pull phi3:mini        # 2.2GB — strong reasoning
```

### Using Ollama in Tests

```rust
// Cargo.toml: langchain-rust = { features = ["ollama"] }

#[tokio::test]
#[cfg_attr(not(feature = "local-llm-tests"), ignore)]
async fn test_chain_with_real_generation() {
    use langchain_rust::llm::ollama::client::Ollama;

    let llm = Ollama::default()
        .with_model("qwen2.5:0.5b")  // smallest available
        .with_base_url("http://localhost:11434");

    let chain = LLMChainBuilder::new()
        .llm(llm)
        .prompt(prompt)
        .build()
        .unwrap();

    let result = chain.invoke(prompt_args! {
        "input" => "Say only the word 'pong'."
    }).await.unwrap();

    assert!(result.to_lowercase().contains("pong"));
}
```

### Feature Flag

Add to `Cargo.toml` to gate local-LLM tests separately from cloud tests:

```toml
[features]
local-llm-tests = ["ollama"]
```

Run local LLM tests:

```bash
cargo test --features local-llm-tests -- --ignored
```

### Model Selection Guide

| Model          | Size  | Use for                                    |
| -------------- | ----- | ------------------------------------------ |
| `qwen2.5:0.5b` | 400MB | basic tool call / echo tests               |
| `llama3.2:1b`  | 1.3GB | chain reasoning, short answers             |
| `phi3:mini`    | 2.2GB | agent tests requiring multi-step reasoning |

Always use the smallest model that passes the test — prefer `qwen2.5:0.5b` by default.
</local-llm>

<rules>
## Testing Rules

- Unit tests: use FakeLLM — no network, no model, deterministic
- Tests needing real generation: use Ollama local models, gate with `#[cfg_attr(not(feature = "local-llm-tests"), ignore)]`
- Cloud API tests: stay `#[ignore]` in `tests/integration/`, require explicit env var
- Remove `#[ignore]` only after replacing the live-API call with FakeLLM or local Ollama
- Use `assert_eq!(fake.call_count(), N)` to verify chain invocation counts
- For error-path tests, implement a `FailingLLM` that always returns `Err(LLMError::...)`
- Never use `OPENAI_API_KEY` or `CLAUDE_API_KEY` in automated CI
  </rules>
