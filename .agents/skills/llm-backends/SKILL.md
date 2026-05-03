---
name: llm-backends
description: Constructing LLM backends (OpenAI, Claude, DeepSeek, Qwen, Ollama) and configuring CallOptions.
---

<oneliner>
All backends implement LLM via builder-style with_* methods. Use CallOptions for
temperature/max_tokens/stop words. API keys come from env vars by default.
</oneliner>

<backends>
## Available Backends

| Backend  | Struct                 | Default env var     | Feature flag |
| -------- | ---------------------- | ------------------- | ------------ |
| OpenAI   | `OpenAI<OpenAIConfig>` | `OPENAI_API_KEY`    | (default)    |
| Claude   | `Claude`               | `CLAUDE_API_KEY`    | (default)    |
| DeepSeek | `Deepseek`             | `DEEPSEEK_API_KEY`  | (default)    |
| Qwen     | `Qwen`                 | `DASHSCOPE_API_KEY` | (default)    |
| Ollama   | `Ollama`               | — (local)           | `ollama`     |

</backends>

<openai>
## OpenAI

```rust
use langchainx::llm::openai::{OpenAI, OpenAIModel};
use async_openai::config::OpenAIConfig;

// Default (reads OPENAI_API_KEY from env)
let llm = OpenAI::default();

// With model
let llm = OpenAI::default().with_model(OpenAIModel::Gpt4.to_string());

// With custom config
let config = OpenAIConfig::default().with_api_key("sk-...");
let llm = OpenAI::new(config).with_model(OpenAIModel::Gpt4o.to_string());
```

</openai>

<claude>
## Claude

```rust
use langchainx::llm::claude::{Claude, ClaudeModel};

// Default (reads CLAUDE_API_KEY from env)
let llm = Claude::new();

// With model — use string form to avoid stale enum variants
let llm = Claude::new()
    .with_model("claude-sonnet-4-6")   // prefer string over enum for new models
    .with_api_key("sk-ant-...");
```

Note: `ClaudeModel` enum only covers models up to claude-3.5-sonnet-20240620. For newer
models (claude-sonnet-4-6, claude-opus-4-6) pass the string directly to `.with_model()`.
</claude>

<deepseek-qwen>
## DeepSeek / Qwen (OpenAI-compatible)

```rust
use langchainx::llm::deepseek::Deepseek;
use langchainx::llm::qwen::Qwen;

let deepseek = Deepseek::default(); // reads DEEPSEEK_API_KEY
let qwen = Qwen::default();         // reads DASHSCOPE_API_KEY
```

Both reuse the OpenAI client with a custom base URL internally.
</deepseek-qwen>

<call-options>
## CallOptions

`CallOptions` configures inference parameters. Pass to a builder via `.options()` on chains,
or directly to the LLM via `.with_options()`.

```rust
use langchainx::language_models::options::CallOptions;

let options = CallOptions::new()
    .with_max_tokens(512)
    .with_temperature(0.7)
    .with_top_p(0.9)
    .with_stop_words(vec!["END".to_string()]);

// Apply to LLM directly
let llm = Claude::new().with_options(options.clone());

// Or via chain builder (preferred — chain merges into LLM options)
use langchainx::chain::options::ChainCallOptions;
let chain_opts = ChainCallOptions::default()
    .with_max_tokens(512)
    .with_temperature(0.7);

let chain = LLMChainBuilder::new()
    .llm(Claude::new())
    .prompt(prompt)
    .options(chain_opts)
    .build()?;
```

DO NOT set `streaming_func` on CallOptions — use `chain.stream()` or `llm.stream()` instead.
See the `streaming` skill.
</call-options>

<add-options>
## Overriding Options at Runtime

`LLM::add_options` merges options in-place. Used internally by chain builders.

```rust
let mut llm = OpenAI::default();
llm.add_options(CallOptions::new().with_temperature(0.0));
```

</add-options>
