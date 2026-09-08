# Design: Shared Conformance Harness and TODO Closure

## Goal

Provide reusable contract tests for every public langchainx trait while closing the remaining
actionable inline TODOs with unit, property, fuzz, and regression coverage.

## Approved Approach

Create a public `langchainx-testsuite` crate, migrate all root conformance suites to thin test
registrations, and apply focused fixes in each owning crate.

## Context Map

### Files to Modify

| Area | Files | Change |
| --- | --- | --- |
| Workspace | `Cargo.toml`, `Cargo.lock` | Register and consume `langchainx-testsuite` |
| Harness | `crates/langchainx-testsuite/**` | Add contracts and deterministic test doubles |
| Conformance | `tests/conformance_*.rs`, `tests/common/mod.rs` | Delegate shared assertions and fakes |
| Conversions | `crates/langchainx-llm/src/schemas/*.rs` | Add reverse conversions and round-trip cases |
| LLM behavior | `crates/langchainx-llm/src/ollama/client.rs` | Ignore metadata-only stream chunks |
| Unit/property tests | owning crate modules, `tests/property_tests.rs` | Add missing behavior coverage |
| Fuzzing | `fuzz/Cargo.toml`, `fuzz/fuzz_targets/*.rs` | Add parser and HTML loader targets |

### Dependencies

`langchainx-testsuite` depends directly on component crates and never on the root `langchainx`
facade. The facade uses it only as a development dependency, so no dependency cycle is created.

### Existing Coverage

Nine root integration suites already define contracts for Agent, Chain, Embedder, Loader, LLM,
Memory, OutputParser, Prompt, and Tool. Their assertions are reusable but currently private to
individual test binaries. Shared doubles currently live in `tests/common/mod.rs`.

### Risk

- The testsuite is a new public crate and therefore adds a versioned API surface.
- Loader contracts must remain generic because `Loader::load` consumes `self`.
- DummyMemory has a deliberate no-op contract separate from mutable memory implementations.
- Provider-specific behavior remains in component crates; the harness performs no network I/O.

## Crate Ownership

- **Owner crate**: `langchainx-testsuite` owns reusable assertions and deterministic doubles.
- **Root facade**: registers concrete implementations as test cases.
- **Component crates**: retain production behavior and provider conversion implementations.

## Public API

### Contract Modules

```rust
pub mod contracts {
    pub mod agent;
    pub mod chain;
    pub mod conversion;
    pub mod embedder;
    pub mod loader;
    pub mod llm;
    pub mod memory;
    pub mod output_parser;
    pub mod prompt;
    pub mod tool;
}
```

### Agent Contracts

```rust
pub async fn assert_plan_returns_event(agent: &dyn Agent, inputs: PromptArgs);
pub fn assert_tool_names(agent: &dyn Agent, expected: &[&str]);
```

Executor loop behavior remains in the root integration suite because it tests `AgentExecutor`,
not the `Agent` trait contract.

### Chain Contracts

```rust
pub async fn assert_call_generation(chain: &dyn Chain, inputs: PromptArgs, expected: &str);
pub async fn assert_invoke_generation(chain: &dyn Chain, inputs: PromptArgs, expected: &str);
pub async fn assert_execute_output(chain: &dyn Chain, inputs: PromptArgs, expected: &str);
pub async fn assert_missing_input(chain: &dyn Chain, inputs: PromptArgs);
pub fn assert_output_keys(chain: &dyn Chain, expected: &[&str]);
```

### Conversion Contracts

```rust
pub fn assert_openai_round_trip<L, O>(value: L)
where
    L: Clone + Debug + PartialEq + LangchainIntoOpenAI<O>,
    O: OpenAiIntoLangchain<L>;
```

### Embedder Contracts

```rust
pub async fn assert_embedder_contract(embedder: &dyn Embedder);
pub async fn assert_empty_documents_contract(embedder: &dyn Embedder);
```

### Loader Contracts

```rust
pub async fn collect_documents<L: Loader>(loader: L) -> Vec<Document>;
pub async fn assert_loader_contents<L: Loader>(loader: L, expected: &[&str]);
```

### LLM Contracts

```rust
pub async fn assert_generate_contract(llm: &dyn LLM, expected: &str);
pub async fn assert_invoke_contract(llm: &dyn LLM, expected: &str);
pub fn assert_message_format_contract(llm: &dyn LLM, messages: &[Message], expected: &str);
```

### Memory Contracts

```rust
pub fn assert_memory_contract<M: BaseMemory>(memory: M);
pub fn assert_noop_memory_contract<M: BaseMemory>(memory: M);
pub fn assert_memory_format_contract<M: BaseMemory>(memory: M, expected: &str);
```

### Output Parser Contracts

```rust
pub async fn assert_parse_contract<P: OutputParser>(parser: &P, input: &str, expected: &str);
pub async fn assert_deterministic_parse<P: OutputParser>(parser: &P, input: &str);
```

### Prompt Contracts

```rust
pub fn assert_prompt_format<F: PromptFromatter>(
    formatter: &F,
    inputs: PromptArgs,
    expected: &str,
);
pub fn assert_prompt_variables<F: PromptFromatter>(formatter: &F, expected: &[&str]);
pub fn assert_missing_prompt_input<F: PromptFromatter>(formatter: &F, inputs: PromptArgs);
```

Concrete `FormatPrompter` and `MessageFormatter` cases remain root registrations because their
associated output shapes are implementation-specific.

### Tool Contracts

```rust
pub async fn assert_tool_contract(tool: Arc<dyn Tool>, valid_input: &str);
pub async fn assert_default_parse_input(tool: Arc<dyn Tool>);
```

### Shared Fakes

```rust
#[derive(Clone, Debug)]
pub struct FakeLLM {
    responses: Arc<Mutex<VecDeque<String>>>,
    call_count: Arc<AtomicUsize>,
}

impl FakeLLM {
    pub fn new(responses: impl IntoIterator<Item = impl Into<String>>) -> Self;
    pub fn call_count(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct FakeEmbedder {
    dimensions: usize,
}

impl FakeEmbedder {
    pub fn new(dimensions: usize) -> Self;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EchoTool;

#[derive(Clone)]
pub struct ScriptedAgent {
    events: Arc<Mutex<VecDeque<AgentEvent>>>,
    tools: Vec<Arc<dyn Tool>>,
}

impl ScriptedAgent {
    pub fn new(events: Vec<AgentEvent>, tools: Vec<Arc<dyn Tool>>) -> Self;
    pub fn finishing(output: impl Into<String>) -> Self;
}
```

## Data Flow

1. A root integration test constructs a concrete implementation or shared fake.
2. The test passes it to a `langchainx-testsuite::contracts` assertion.
3. The assertion invokes only the public trait contract and reports invariant failures.
4. Provider-specific round trips convert a langchainx value to an OpenAI type and back before
   equality is asserted.

## TODO Closure

- Ollama streaming filters chunks without message content.
- Reverse OpenAI conversions support lossless variants and are round-trip tested.
- LLM, memory, Tool, CSV, and Embedder contracts gain the missing coverage.
- MarkdownParser and HtmlLoader gain registered fuzz targets.
- TODO comments are removed only after their stated behavior is verified.

## Out of Scope

- Jira, Confluence, and BigQuery implementations; each receives a dedicated tracking issue.
- Live provider calls in conformance tests.
- Moving component-crate tests into the testsuite or making component crates depend on it.

## Compatibility

- Breaking API changes: none.
- New external dependencies: none; the crate uses existing workspace dependencies.
- Feature flags: no new runtime flags; environment-dependent helpers remain optional.
