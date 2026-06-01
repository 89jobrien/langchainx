# langchainx

Building applications with LLMs through composability, with Rust.

> **Upstream credit:** This crate is a fork of
> [langchain-rust](https://github.com/Abraxas-365/langchain-rust) by
> [Ryo Kanazawa (Abraxas-365)](https://github.com/Abraxas-365), used under the MIT License.

## What makes this fork different

- **Typed error types** throughout — no more opaque `Box<dyn Error>` chains
- **Cargo workspace** split into 14 focused sub-crates (use only what you need)
- **Three-tier e2e test suite** — offline fakes, local Ollama, and
  [smolvm](https://github.com/smol-machines/smolvm)-based container tests (no Docker)
- **Convenience macros** — `tool!`, `llm!`, `prompt!`, `chain!` via `langchainx-macros`
- **Tree-sitter source code loader** with parsers for 11 languages

## Workspace Crates

The root `langchainx` crate re-exports everything for convenience; individual crates
can be used directly for smaller dependency footprints.

| Crate | Contents |
| --- | --- |
| `langchainx-core` | Shared schemas, document types, error primitives |
| `langchainx-llm` | LLM backends (OpenAI, Claude, DeepSeek, Qwen, Ollama) |
| `langchainx-prompt` | Prompt templates and formatting macros |
| `langchainx-chain` | Chain types (LLMChain, Conversational, Sequential, SQL, Q&A) |
| `langchainx-agent` | Agent abstractions and executors (Chat, OpenAI Tools) |
| `langchainx-memory` | Conversation memory backends (SimpleMemory, WindowBuffer) |
| `langchainx-embedding` | Embedding backends (OpenAI, Ollama, FastEmbed, MistralAI) |
| `langchainx-output-parsers` | Output parsers and structured extraction |
| `langchainx-tools` | Tool trait and built-in tools (search, command, scraper, SQL) |
| `langchainx-loaders` | Document loaders (PDF, HTML, CSV, Pandoc, Git, source code) |
| `langchainx-vectorstore` | Vector store backends (Postgres, Qdrant, OpenSearch, SQLite, SurrealDB) |
| `langchainx-text-splitter` | Text splitting utilities (token-aware, markdown-aware) |
| `langchainx-semantic-router` | Semantic routing — static and dynamic (LLM-backed) |
| `langchainx-macros` | Convenience macros: `tool!`, `llm!`, `prompt!`, `chain!` |

## Supported Integrations

### LLMs

- [OpenAI](examples/llm_openai.rs) /
  [Azure OpenAI](examples/llm_azure_open_ai.rs)
- [Anthropic Claude](examples/llm_anthropic_claude.rs)
- [DeepSeek](examples/llm_deepseek.rs) — OpenAI-compatible, streaming +
  reasoning_content support
  ([advanced](examples/llm_deepseek_advanced.rs))
- [Qwen / Alibaba Cloud](examples/llm_alibaba_qwen.rs) — OpenAI-compatible
  ([advanced](examples/llm_qwen_advanced.rs))
- [Ollama](examples/llm_ollama.rs) (local models)
- [Vision / multimodal](examples/vision_llm_chain.rs)

### Embeddings

- [OpenAI](examples/embedding_openai.rs) /
  [Azure OpenAI](examples/embedding_azure_open_ai.rs)
- [Ollama](examples/embedding_ollama.rs)
- [FastEmbed](examples/embedding_fastembed.rs) (local, no API key)
- [MistralAI](examples/embedding_mistralai.rs)

### Vector Stores

- [Postgres (pgvector)](examples/vector_store_postgres.rs)
- [Qdrant](examples/vector_store_qdrant.rs)
- [OpenSearch](examples/vector_store_opensearch.rs)
- [SQLite (sqlite-vss)](examples/vector_store_sqlite_vss.rs) /
  [SQLite (sqlite-vec)](examples/vector_store_sqlite_vec.rs)
- [SurrealDB](examples/vector_store_surrealdb/src/main.rs)

### Chains

- [LLM Chain](examples/llm_chain.rs) /
  [Simple Chain](examples/simple_chain.rs) /
  [Streaming](examples/streaming_from_chain.rs)
- [Conversational](examples/conversational_chain.rs) /
  [Conversational Retriever](examples/conversational_retriever_simple_chain.rs) /
  [with Vector Store](examples/conversational_retriever_chain_with_vector_store.rs)
- [Sequential Chain](examples/sequential_chain.rs)
- [Q&A Chain](examples/qa_chain.rs) /
  [SQL Chain](examples/sql_chain.rs)
- [DeepSeek Chain](examples/llm_chain_deepseek.rs) /
  [Qwen Chain](examples/llm_chain_qwen.rs)

### Agents

- [Chat Agent with Tools](examples/agent.rs)
- [OpenAI Tools Agent](examples/open_ai_tools_agent.rs)
- [AI Commit Message Generator](examples/rcommiter.rs) — reads
  `git diff --staged` and generates a conventional commit message

### Tools

- Serpapi / Google search, DuckDuckGo search
- [Wolfram / Math](examples/wolfram_tool.rs)
- Command line executor
- [Text-to-Speech](examples/text_to_speech.rs) /
  [Speech-to-Text (Whisper)](examples/speech2text_openai.rs)

### Semantic Routing

- [Static routing](examples/semantic_routes.rs)
- [Dynamic routing](examples/dynamic_semantic_routes.rs) (LLM-backed)

### Document Loaders

PDF, HTML, [HTML-to-Markdown](examples/), CSV, Pandoc (DOCX, etc.), Git commits,
source code (tree-sitter with C, C++, C#, Go, Java, JavaScript, Kotlin, Python,
Rust, Scala, TypeScript)

## Testing

Three-tier e2e test suite. All tiers skip gracefully when prerequisites are
unavailable.

| Tier | File | Prerequisite | Command |
| --- | --- | --- | --- |
| 1 -- Offline | `tests/e2e_offline.rs` | None (FakeLLM / FakeEmbedder) | `cargo test --test e2e_offline` |
| 2 -- Local LLM | `tests/e2e_local_llm.rs` | Ollama + `qwen2.5:0.5b` | `cargo test --test e2e_local_llm --features ollama` |
| 3 -- Containers | `tests/e2e_containers.rs` | [smolvm](https://github.com/smol-machines/smolvm) on PATH | `cargo test --test e2e_containers --features postgres,qdrant` |

Tier 1 always passes in CI. Tier 2 skips when Ollama is unavailable. Tier 3
spins up real Postgres/pgvector and Qdrant VMs via smolvm -- no Docker daemon
required.

```bash
# Pull the model for tier 2
ollama pull qwen2.5:0.5b

# Run all tiers
cargo test --all-features
```

## Installation

### Step 1: Add `serde_json`

```bash
cargo add serde_json
```

### Step 2: Add `langchainx`

```bash
cargo add langchainx
```

### With optional backends

```bash
cargo add langchainx --features postgres     # pgvector
cargo add langchainx --features qdrant
cargo add langchainx --features surrealdb
cargo add langchainx --features opensearch
cargo add langchainx --features sqlite-vss   # requires sqlite-vss libraries
cargo add langchainx --features sqlite-vec   # requires sqlite-vec libraries
cargo add langchainx --features ollama
cargo add langchainx --features fastembed    # local embeddings, no API key
```

SQLite extensions: [sqlite-vss](https://github.com/asg017/sqlite-vss),
[sqlite-vec](https://github.com/asg017/sqlite-vec).

## Feature Flags

All integrations are opt-in. Default features are empty -- only core
OpenAI/Claude/DeepSeek/Qwen work without flags.

| Flag | What it enables |
| --- | --- |
| `postgres` | Postgres/pgvector vector store + SQL chain |
| `qdrant` | Qdrant vector store |
| `surrealdb` | SurrealDB vector store |
| `opensearch` | OpenSearch vector store |
| `sqlite-vss` | SQLite vector store (Faiss-based) |
| `sqlite-vec` | SQLite vector store (pure C, portable) |
| `ollama` | Ollama LLM + embedding backend |
| `fastembed` | Local FastEmbed embeddings |
| `mistralai` | MistralAI embedding backend |
| `git` | Git commit document loader |
| `lopdf` / `pdf-extract` | PDF document loaders |
| `html-to-markdown` | HTML-to-Markdown document loader |
| `tree-sitter` | Source code loader with 11 language parsers |
| `rss` / `sitemap` | RSS and sitemap document loaders |

## Quick Start

```rust
use langchainx::{
    chain::{Chain, LLMChainBuilder},
    fmt_message, fmt_placeholder, fmt_template,
    language_models::llm::LLM,
    llm::openai::{OpenAI, OpenAIModel},
    message_formatter,
    prompt::HumanMessagePromptTemplate,
    prompt_args,
    schemas::messages::Message,
    template_fstring,
};

#[tokio::main]
async fn main() {
    let open_ai = OpenAI::default()
        .with_model(OpenAIModel::Gpt4oMini.to_string());

    // Simple invocation
    let resp = open_ai.invoke("What is rust").await.unwrap();
    println!("{}", resp);

    // With a prompt template
    let prompt = message_formatter![
        fmt_message!(Message::new_system_message(
            "You are world class technical documentation writer."
        )),
        fmt_template!(HumanMessagePromptTemplate::new(
            template_fstring!("{input}", "input")
        ))
    ];

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(open_ai.clone())
        .build()
        .unwrap();

    let result = chain
        .invoke(prompt_args! { "input" => "What is LangChain?" })
        .await
        .unwrap();
    println!("Result: {:?}", result);

    // With conversation history
    let prompt = message_formatter![
        fmt_message!(Message::new_system_message(
            "You are world class technical documentation writer."
        )),
        fmt_placeholder!("history"),
        fmt_template!(HumanMessagePromptTemplate::new(
            template_fstring!("{input}", "input")
        )),
    ];

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(open_ai)
        .build()
        .unwrap();

    let result = chain
        .invoke(prompt_args! {
            "input" => "Who is the writer of 20,000 Leagues Under the Sea, \
                        and what is my name?",
            "history" => vec![
                Message::new_human_message("My name is: luis"),
                Message::new_ai_message("Hi luis"),
            ],
        })
        .await
        .unwrap();
    println!("Result: {:?}", result);
}
```
