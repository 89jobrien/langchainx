# langchainx

Building applications with LLMs through composability, with Rust!

> **Upstream credit:** This crate is a fork of
> [langchain-rust](https://github.com/Abraxas-365/langchain-rust) by
> [Ryo Kanazawa (Abraxas-365)](https://github.com/Abraxas-365), used under the MIT License.
> Changes in this fork include typed error types, a three-tier e2e test suite, smolvm-based
> container tests, a Cargo workspace split into focused sub-crates, and ongoing architectural
> improvements tracked in [GitHub Issues](https://github.com/89jobrien/langchainx/issues).

## What is this?

This is the Rust language implementation of [LangChain](https://github.com/langchain-ai/langchain).

## Workspace Crates

The library is organized as a Cargo workspace. The root `langchainx` crate re-exports everything
for convenience; individual crates can be used directly for smaller dependency footprints.

| Crate                      | Contents                                              |
| -------------------------- | ----------------------------------------------------- |
| `langchainx-core`          | Shared schemas, document types, error primitives      |
| `langchainx-llm`           | LLM backends (OpenAI, Claude, DeepSeek, Qwen, Ollama) |
| `langchainx-prompt`        | Prompt templates and formatting macros                |
| `langchainx-memory`        | Conversation memory backends                          |
| `langchainx-embedding`     | Embedding backends (OpenAI, Ollama, FastEmbed, etc.)  |
| `langchainx-output-parsers`| Output parsers and structured extraction              |
| `langchainx-chain`         | Chain types (LLMChain, ConversationalChain, etc.)     |

## Current Features

- LLMs
    - [x] [OpenAI](https://github.com/89jobrien/langchainx/blob/main/examples/llm_openai.rs)
    - [x] [Azure OpenAI](https://github.com/89jobrien/langchainx/blob/main/examples/llm_azure_open_ai.rs)
    - [x] [Ollama](https://github.com/89jobrien/langchainx/blob/main/examples/llm_ollama.rs)
    - [x] [Anthropic Claude](https://github.com/89jobrien/langchainx/blob/main/examples/llm_anthropic_claude.rs)
    - [x] [DeepSeek](https://github.com/89jobrien/langchainx/blob/main/examples/llm_deepseek.rs)
      (OpenAI-compatible, streaming + reasoning_content support;
      [advanced](https://github.com/89jobrien/langchainx/blob/main/examples/llm_deepseek_advanced.rs))
    - [x] [Qwen / Alibaba Cloud](https://github.com/89jobrien/langchainx/blob/main/examples/llm_alibaba_qwen.rs)
      (OpenAI-compatible;
      [advanced](https://github.com/89jobrien/langchainx/blob/main/examples/llm_qwen_advanced.rs))
    - [x] [Vision / multimodal LLM chain](https://github.com/89jobrien/langchainx/blob/main/examples/vision_llm_chain.rs)

- Embeddings
    - [x] [OpenAI](https://github.com/89jobrien/langchainx/blob/main/examples/embedding_openai.rs)
    - [x] [Azure OpenAI](https://github.com/89jobrien/langchainx/blob/main/examples/embedding_azure_open_ai.rs)
    - [x] [Ollama](https://github.com/89jobrien/langchainx/blob/main/examples/embedding_ollama.rs)
    - [x] [Local FastEmbed](https://github.com/89jobrien/langchainx/blob/main/examples/embedding_fastembed.rs)
    - [x] [MistralAI](https://github.com/89jobrien/langchainx/blob/main/examples/embedding_mistralai.rs)

- VectorStores
    - [x] [OpenSearch](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_opensearch.rs)
    - [x] [Postgres](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_postgres.rs)
    - [x] [Qdrant](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_qdrant.rs)
    - [x] [SQLite (sqlite-vss)](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_sqlite_vss.rs)
    - [x] [SQLite (sqlite-vec)](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_sqlite_vec.rs)
    - [x] [SurrealDB](https://github.com/89jobrien/langchainx/blob/main/examples/vector_store_surrealdb/src/main.rs)

- Chain
    - [x] [LLM Chain](https://github.com/89jobrien/langchainx/blob/main/examples/llm_chain.rs)
    - [x] [Simple Chain](https://github.com/89jobrien/langchainx/blob/main/examples/simple_chain.rs)
    - [x] [Streaming from Chain](https://github.com/89jobrien/langchainx/blob/main/examples/streaming_from_chain.rs)
    - [x] [LLM Chain — DeepSeek](https://github.com/89jobrien/langchainx/blob/main/examples/llm_chain_deepseek.rs)
    - [x] [LLM Chain — Qwen](https://github.com/89jobrien/langchainx/blob/main/examples/llm_chain_qwen.rs)
    - [x] [Conversational Chain](https://github.com/89jobrien/langchainx/blob/main/examples/conversational_chain.rs)
    - [x] [Conversational Retriever Simple](https://github.com/89jobrien/langchainx/blob/main/examples/conversational_retriever_simple_chain.rs)
    - [x] [Conversational Retriever With Vector Store](https://github.com/89jobrien/langchainx/blob/main/examples/conversational_retriever_chain_with_vector_store.rs)
    - [x] [Sequential Chain](https://github.com/89jobrien/langchainx/blob/main/examples/sequential_chain.rs)
    - [x] [Q&A Chain](https://github.com/89jobrien/langchainx/blob/main/examples/qa_chain.rs)
    - [x] [SQL Chain](https://github.com/89jobrien/langchainx/blob/main/examples/sql_chain.rs)

- Agents
    - [x] [Chat Agent with Tools](https://github.com/89jobrien/langchainx/blob/main/examples/agent.rs)
    - [x] [OpenAI Tools Agent](https://github.com/89jobrien/langchainx/blob/main/examples/open_ai_tools_agent.rs)
    - [x] [AI Commit Message Generator](https://github.com/89jobrien/langchainx/blob/main/examples/rcommiter.rs)
      — reads `git diff --staged` and generates a conventional commit message

- Tools
    - [x] Serpapi/Google
    - [x] DuckDuckGo Search
    - [x] [Wolfram/Math](https://github.com/89jobrien/langchainx/blob/main/examples/wolfram_tool.rs)
    - [x] Command line
    - [x] [Text-to-Speech](https://github.com/89jobrien/langchainx/blob/main/examples/text_to_speech.rs)
    - [x] [Speech-to-Text (OpenAI Whisper)](https://github.com/89jobrien/langchainx/blob/main/examples/speech2text_openai.rs)

- Semantic Routing
    - [x] [Static Routing](https://github.com/89jobrien/langchainx/blob/main/examples/semantic_routes.rs)
    - [x] [Dynamic Routing](https://github.com/89jobrien/langchainx/blob/main/examples/dynamic_semantic_routes.rs)

- Document Loaders
    - [x] PDF

        ```rust
        use futures_util::StreamExt;

        async fn main() {
            let path = "./src/document_loaders/test_data/sample.pdf";

            let loader = PdfExtractLoader::from_path(path).expect("Failed to create PdfExtractLoader");
            // let loader = LoPdfLoader::from_path(path).expect("Failed to create LoPdfLoader");

            let docs = loader
                .load()
                .await
                .unwrap()
                .map(|d| d.unwrap())
                .collect::<Vec<_>>()
                .await;

        }
        ```

    - [x] Pandoc

        ```rust
        use futures_util::StreamExt;

        async fn main() {

            let path = "./src/document_loaders/test_data/sample.docx";

            let loader = PandocLoader::from_path(InputFormat::Docx.to_string(), path)
                .await
                .expect("Failed to create PandocLoader");

            let docs = loader
                .load()
                .await
                .unwrap()
                .map(|d| d.unwrap())
                .collect::<Vec<_>>()
                .await;
        }
        ```

    - [x] HTML

        ```rust
        use futures_util::StreamExt;
        use url::Url;

        async fn main() {
            let path = "./src/document_loaders/test_data/example.html";
            let html_loader = HtmlLoader::from_path(path, Url::parse("https://example.com/").unwrap())
                .expect("Failed to create html loader");

            let documents = html_loader
                .load()
                .await
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()
                .await;
        }
        ```

    - [x] HTML To Markdown

        ```rust
        use futures_util::StreamExt;
        use url::Url;

        async fn main() {
            let path = "./src/document_loaders/test_data/example.html";
            let html_to_markdown_loader = HtmlToMarkdownLoader::from_path(path, Url::parse("https://example.com/").unwrap(), HtmlToMarkdownOptions::default().with_skip_tags(vec!["figure".to_string()]))
                .expect("Failed to create html to markdown loader");

            let documents = html_to_markdown_loader
                .load()
                .await
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()
                .await;
        }
        ```

    - [x] CSV

        ```rust
        use futures_util::StreamExt;

        async fn main() {
            let path = "./src/document_loaders/test_data/test.csv";
            let columns = vec![
                "name".to_string(),
                "age".to_string(),
                "city".to_string(),
                "country".to_string(),
            ];
            let csv_loader = CsvLoader::from_path(path, columns).expect("Failed to create csv loader");

            let documents = csv_loader
                .load()
                .await
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()
                .await;
        }
        ```

    - [x] [Git commits](https://github.com/89jobrien/langchainx/blob/main/examples/git_commits.rs)

        ```rust
        use futures_util::StreamExt;

        async fn main() {
            let path = "/path/to/git/repo";
            let loader = GitCommitLoader::from_path(path).expect("Failed to create GitCommitLoader");

            let documents = loader
                .load()
                .await
                .unwrap()
                .map(|x| x.unwrap())
                .collect::<Vec<_>>()
                .await;
        }
        ```

    - [x] Source code

        ```rust

        let loader_with_dir =
        SourceCodeLoader::from_path("./src/document_loaders/test_data".to_string())
        .with_dir_loader_options(DirLoaderOptions {
        glob: None,
        suffixes: Some(vec!["rs".to_string()]),
        exclude: None,
        });

        let stream = loader_with_dir.load().await.unwrap();
        let documents = stream.map(|x| x.unwrap()).collect::<Vec<_>>().await;
        ```

## Testing

This fork ships a three-tier e2e test suite. All tiers are independent and skip gracefully
when their prerequisites are unavailable.

| Tier           | File                      | Prerequisite                           | Command                                                       |
| -------------- | ------------------------- | -------------------------------------- | ------------------------------------------------------------- |
| 1 — Offline    | `tests/e2e_offline.rs`    | None — uses `FakeLLM`/`FakeEmbedder`   | `cargo test --test e2e_offline`                               |
| 2 — Local LLM  | `tests/e2e_local_llm.rs`  | Ollama running + `qwen2.5:0.5b` pulled | `cargo test --test e2e_local_llm --features ollama`           |
| 3 — Containers | `tests/e2e_containers.rs` | `smolvm` on PATH (no Docker required)  | `cargo test --test e2e_containers --features postgres,qdrant` |

**Tier 1** always passes in CI. Tests verify chain correctness using deterministic fakes.

**Tier 2** skips automatically when Ollama is unavailable. Assertions check doneness (non-empty
output, no panic) rather than exact model responses.

**Tier 3** spins up real Postgres/pgvector and Qdrant VMs via
[smolvm](https://github.com/smolvm/smolvm) — no Docker daemon required. Each test allocates
a free port, starts the VM, runs add→search round-trips, and tears the VM down on drop.

```bash
# Pull the model for tier 2
ollama pull qwen2.5:0.5b

# Run all tiers
cargo test --all-features
```

## Installation

This library heavily relies on `serde_json` for its operation.

### Step 1: Add `serde_json`

First, ensure `serde_json` is added to your Rust project.

```bash
cargo add serde_json
```

### Step 2: Add `langchainx`

Then, you can add `langchainx` to your Rust project.

#### Simple install

```bash
cargo add langchainx
```

#### With Sqlite

##### sqlite-vss

Download additional sqlite_vss libraries from <https://github.com/asg017/sqlite-vss>

```bash
cargo add langchainx --features sqlite-vss
```

##### sqlite-vec

Download additional sqlite_vec libraries from <https://github.com/asg017/sqlite-vec>

```bash
cargo add langchainx --features sqlite-vec
```

#### With Postgres

```bash
cargo add langchainx --features postgres
```

#### With SurrealDB

```bash
cargo add langchainx --features surrealdb
```

#### With Qdrant

```bash
cargo add langchainx --features qdrant
```

Please remember to replace the feature flags `sqlite`, `postgres` or `surrealdb` based on your
specific use case.

This will add both `serde_json` and `langchainx` as dependencies in your `Cargo.toml`
file. Now, when you build your project, both dependencies will be fetched and compiled, and will be available for use in your project.

Remember, `serde_json` is a necessary dependencies, and `sqlite`, `postgres` and `surrealdb`
are optional features that may be added according to project needs.

### Quick Start Conversational Chain

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
    //We can then initialize the model:
    // If you'd prefer not to set an environment variable you can pass the key in directly via the `openai_api_key` named parameter when initiating the OpenAI LLM class:
    // let open_ai = OpenAI::default()
    //     .with_config(
    //         OpenAIConfig::default()
    //             .with_api_key("<your_key>"),
    //     ).with_model(OpenAIModel::Gpt4oMini.to_string());
    let open_ai = OpenAI::default().with_model(OpenAIModel::Gpt4oMini.to_string());


    //Once you've installed and initialized the LLM of your choice, we can try using it! Let's ask it what LangSmith is - this is something that wasn't present in the training data so it shouldn't have a very good response.
    let resp = open_ai.invoke("What is rust").await.unwrap();
    println!("{}", resp);

    // We can also guide it's response with a prompt template. Prompt templates are used to convert raw user input to a better input to the LLM.
    let prompt = message_formatter![
        fmt_message!(Message::new_system_message(
            "You are world class technical documentation writer."
        )),
        fmt_template!(HumanMessagePromptTemplate::new(template_fstring!(
            "{input}", "input"
        )))
    ];

    //We can now combine these into a simple LLM chain:

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(open_ai.clone())
        .build()
        .unwrap();

    //We can now invoke it and ask the same question. It still won't know the answer, but it should respond in a more proper tone for a technical writer!

    match chain
        .invoke(prompt_args! {
        "input" => "Quien es el escritor de 20000 millas de viaje submarino",
           })
        .await
    {
        Ok(result) => {
            println!("Result: {:?}", result);
        }
        Err(e) => panic!("Error invoking LLMChain: {:?}", e),
    }

    //If you want to prompt to have a list of messages you could use the `fmt_placeholder` macro

    let prompt = message_formatter![
        fmt_message!(Message::new_system_message(
            "You are world class technical documentation writer."
        )),
        fmt_placeholder!("history"),
        fmt_template!(HumanMessagePromptTemplate::new(template_fstring!(
            "{input}", "input"
        ))),
    ];

    let chain = LLMChainBuilder::new()
        .prompt(prompt)
        .llm(open_ai)
        .build()
        .unwrap();
    match chain
        .invoke(prompt_args! {
        "input" => "Who is the writer of 20,000 Leagues Under the Sea, and what is my name?",
        "history" => vec![
                Message::new_human_message("My name is: luis"),
                Message::new_ai_message("Hi luis"),
                ],

        })
        .await
    {
        Ok(result) => {
            println!("Result: {:?}", result);
        }
        Err(e) => panic!("Error invoking LLMChain: {:?}", e),
    }
}
```
