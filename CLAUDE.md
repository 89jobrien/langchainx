# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build with all features (matches CI)
cargo build --all-features

# Run tests
cargo test --all-features

# Run a single test
cargo test --all-features <test_name>

# Format check (CI gate)
cargo fmt --all -- --check

# Format
cargo fmt --all

# Run a specific example
cargo run --example llm_chain --all-features
```

CI runs `cargo fmt --check`, then `cargo build --release --all-features`, then `cargo test --release --all-features`. Match this locally before pushing.

## Architecture

This is a Rust port of LangChain. The core abstractions are traits in `src/language_models/`, `src/chain/`, `src/tools/`, `src/embedding/`, and `src/vectorstore/`. All major components implement async traits via `async-trait`.

**Core trait layer** (`src/language_models/llm.rs`, `src/chain/chain_trait.rs`, `src/tools/tool.rs`):

- `LLM` — `generate(&[Message]) -> GenerateResult`, `invoke(&str) -> String`, `stream()`
- `Chain` — `call(PromptArgs) -> GenerateResult`, `invoke(PromptArgs) -> String`, `execute()`, `stream()`
- `Tool` — `name()`, `description()`, `parameters()` (OpenAI function-call JSON schema), `run(Value)`

**LLM backends** (`src/llm/`): OpenAI (via `async-openai`), Claude, DeepSeek, Qwen, Ollama. Each backend has a `client.rs`, `models.rs`, and `error.rs`. OpenAI-compatible APIs (DeepSeek, Qwen) reuse the OpenAI client with a custom base URL.

**Chains** (`src/chain/`): `LLMChain` (basic prompt→LLM), `ConversationalChain` (with memory),
`ConversationalRetrievalQA` (memory + retriever), `SequentialChain` (chains in series),
`StuffDocuments` (stuffs retrieved docs into prompt), `SqlDatabase`. Each has a `builder.rs`
using the builder pattern.

**Memory** (`src/memory/`): `SimpleMemory`, `WindowBufferMemory`, `DummyMemory`. Memory is passed
into chain builders as `Arc<dyn BaseMemory>`.

**Embeddings** (`src/embedding/`): `Embedder` trait. Backends: OpenAI, Azure OpenAI, Ollama,
FastEmbed (local), MistralAI — each behind a Cargo feature flag.

**Vector stores** (`src/vectorstore/`): `VectorStore` trait. Backends: Postgres (pgvector),
Qdrant, OpenSearch, SQLite (sqlite-vss or sqlite-vec), SurrealDB — each behind a feature flag.

**Document loaders** (`src/document_loaders/`): PDF, HTML, CSV, Pandoc, Git commits, source code.
All implement a `Loader` trait returning `Stream<Item = Document>`.

**Agents** (`src/agent/`): `AgentExecutor` runs a tool-use loop. Two agent types: `ChatAgent`
(ReAct-style, text parsing) and `OpenAIToolsAgent` (function-calling API).

**Prompt system** (`src/prompt/`): `message_formatter!`, `fmt_message!`, `fmt_template!`,
`fmt_placeholder!`, `template_fstring!`, `prompt_args!` macros compose prompt templates. Input
variables are passed as `HashMap<String, Value>` (`PromptArgs`).

**Semantic routing** (`src/semantic_router/`): Routes inputs to handlers based on embedding
similarity. Static (fixed routes) and dynamic (LLM-backed) variants.

## Local-Only Files

`.ctx/private/` is gitignored and read-only — meeting transcripts, parsed JSON, summaries,
scratch data. Never commit or import from it. See godmode skill `private-ctx` for the full
convention.

## Feature Flags

Most integrations are opt-in. Key flags: `postgres`, `qdrant`, `surrealdb`, `opensearch`,
`sqlite-vss`, `sqlite-vec`, `ollama`, `fastembed`, `mistralai`, `git`, `lopdf`, `pdf-extract`,
`html-to-markdown`, `tree-sitter`. Default features are empty — only core OpenAI/Claude/DeepSeek/
Qwen work without flags.
