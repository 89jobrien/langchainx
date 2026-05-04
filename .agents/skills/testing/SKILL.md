---
name: testing
description: FakeLLM, in-process test doubles, writing offline chain/agent tests.
---

<oneliner>
FakeLLM is in src/test_utils/ and already implemented. Use it for offline chain tests.
Never call real APIs in unit tests. FakeLLM::new takes Vec<String>.
</oneliner>

<fakellm>
## FakeLLM

`FakeLLM` is already implemented at `src/test_utils/fake_llm.rs` and re-exported from
`langchainx::test_utils::FakeLLM`.

```rust
use langchainx::test_utils::FakeLLM;

let llm = FakeLLM::new(vec![
    "Hello from FakeLLM!".to_string(),
    "Second response".to_string(),
]);

// Responses are popped in order; empty string when exhausted
assert_eq!(llm.invoke("anything").await.unwrap(), "Hello from FakeLLM!");
assert_eq!(llm.call_count(), 1);
assert_eq!(llm.remaining(), 1);
```

Key details:
- `FakeLLM::new` takes `Vec<String>` (not `Vec<&str>`)
- `.clone()` shares the same queue — both clones see the same state
- `.stream()` returns `Err` — use `FakeLLM` only for `generate`/`invoke` tests
- Uses `std::sync::Mutex` (not tokio) for the response queue
</fakellm>

<writing-tests>
## Writing Offline Chain Tests

```rust
#[cfg(test)]
mod tests {
    use langchainx::{
        chain::{Chain, LLMChainBuilder},
        message_formatter, fmt_template,
        prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
        prompt_args, template_fstring,
        test_utils::FakeLLM,
    };

    #[tokio::test]
    async fn test_llm_chain_invoke() {
        let fake = FakeLLM::new(vec!["Hello from FakeLLM!".to_string()]);

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
    async fn test_chain_returns_empty_when_no_responses() {
        let fake = FakeLLM::new(vec![]); // no responses queued
        // ... build chain ...
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
// Cargo.toml: langchainx = { features = ["ollama"] }

#[tokio::test]
#[cfg_attr(not(feature = "local-llm-tests"), ignore)]
async fn test_chain_with_real_generation() {
    use langchainx::llm::ollama::client::Ollama;

    let llm = Ollama::default()
        .with_model("qwen2.5:0.5b")
        .with_base_url("http://localhost:11434");

    let chain = LLMChainBuilder::new()
        .llm(llm)
        .prompt(prompt)
        .build()
        .unwrap();

    let result = chain.invoke(prompt_args! {
        "input" => "Say only the word 'pong'."
    }).await.unwrap();

    assert!(!result.is_empty(), "model returned no output");
}
```

### Feature Flag

```toml
[features]
local-llm-tests = ["ollama"]
```

Run local LLM tests:

```bash
cargo test --features local-llm-tests -- --ignored
```

### What to Assert

Local LLM tests should verify **doneness, not correctness** — the chain completed without
error and returned something, not that it returned a specific string.

```rust
// WRONG — brittle, model-dependent phrasing
assert_eq!(result, "The capital of France is Paris.");

// CORRECT — verify task completion
assert!(!result.is_empty(), "model returned no output");
assert!(result.len() > 10, "response too short to be a real answer");

// CORRECT — for agent/tool tests, verify the tool was called
assert!(!result.is_empty(), "agent should have produced a final answer");
```

### Model Selection Guide

| Model          | Size  | Use for                                     |
| -------------- | ----- | ------------------------------------------- |
| `qwen2.5:0.5b` | 400MB | basic doneness checks (non-empty, no panic) |
| `llama3.2:1b`  | 1.3GB | chain flow tests, multi-turn memory         |
| `phi3:mini`    | 2.2GB | agent tool-use loop tests                   |

Always use the smallest model that passes the test.
</local-llm>

<rules>
## Testing Rules

- Unit tests: use FakeLLM — no network, no model, deterministic
- Tests needing real generation: use Ollama local models, gate with
  `#[cfg_attr(not(feature = "local-llm-tests"), ignore)]`
- Cloud API tests: stay `#[ignore]` in `tests/integration/`, require explicit env var
- Remove `#[ignore]` only after replacing the live-API call with FakeLLM or local Ollama
- Use `assert_eq!(fake.call_count(), N)` to verify chain invocation counts
- For error-path tests, implement a struct that returns `Err(LLMError::...)` from `generate`
- Never use `OPENAI_API_KEY` or `CLAUDE_API_KEY` in automated CI
</rules>
