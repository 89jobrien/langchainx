# langchainx Agent Skills

This project is a Rust port of LangChain. Skills below document correct patterns for this
codebase. **Invoke the relevant skill BEFORE writing or modifying any code.**

## CRITICAL: Invoke Skills Before Writing Code

Skills contain accurate imports, trait bounds, builder patterns, and anti-patterns sourced
directly from the codebase. Without them you will use stale patterns.

## Skill Index

### Core Patterns

| Skill          | Invoke When                                                               |
| -------------- | ------------------------------------------------------------------------- |
| `fundamentals` | Starting any task; LLMChain, builder pattern, basic invocation            |
| `llm-backends` | Constructing an LLM (OpenAI, Claude, DeepSeek, Qwen, Ollama), CallOptions |
| `chains`       | Working with any Chain type or the Chain trait                            |
| `tools`        | Implementing a Tool, calling tools from agents                            |
| `agents`       | AgentExecutor, ChatAgent, OpenAIToolsAgent, Agent trait                   |
| `memory`       | BaseMemory, SimpleMemory, WindowBufferMemory, Arc<Mutex> patterns         |
| `rag`          | VectorStore, Embedder, document loaders, ConversationalRetrievalQA        |

### Architectural Issues (JOB-251–257)

| Skill                 | Invoke When                                                             |
| --------------------- | ----------------------------------------------------------------------- |
| `arc-dyn-llm`         | Touching LLM trait bounds, cloning LLMs, Box<dyn LLM> (JOB-251)         |
| `async-traits`        | Adding or removing async_trait, writing new async trait impls (JOB-252) |
| `streaming`           | Implementing or consuming streaming LLM/chain output (JOB-253)          |
| `prompt-args`         | Passing or validating PromptArgs, adding chain inputs (JOB-254)         |
| `error-handling`      | Implementing Tool::run, defining new error types (JOB-255)              |
| `memory-construction` | Adding a new memory type or constructing memory in a builder (JOB-256)  |
| `testing`             | Writing any test for a chain, agent, or LLM (JOB-257)                   |
| `conformance-testing` | Adding conformance suites, proptests, or fuzz targets for any trait     |

## Environment

Required env vars:

```bash
OPENAI_API_KEY=<key>
CLAUDE_API_KEY=<key>      # Anthropic
DEEPSEEK_API_KEY=<key>
```

## Build & Test

```bash
cargo build --all-features
cargo test --all-features
cargo clippy --all-features -- -D warnings
cargo fmt --all -- --check
```

All chain/agent tests are currently `#[ignore]` pending FakeLLM (JOB-257). Run ignored tests
only with a live API key:

```bash
cargo test --all-features -- --ignored
```
