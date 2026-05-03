---
name: conformance-testing
description: Trait conformance tests, property-based tests (proptest), fuzz targets, and boundary contracts for langchain-rust traits.
---

<oneliner>
Every public trait (LLM, Chain, Tool, Embedder, VectorStore, BaseMemory) needs a
conformance test suite. Use proptest for invariant testing, cargo-fuzz for input fuzzing.
</oneliner>

<conformance-suites>
## Trait Conformance Test Suites

A conformance suite verifies that any implementation of a trait obeys its contract.
Place these in `tests/conformance/<trait_name>.rs`.

### Pattern

```rust
// tests/conformance/tool.rs
//! Conformance tests for the Tool trait.
//! Any Tool impl can be tested by instantiating conformance_suite(tool).

use langchain_rust::tools::Tool;

pub async fn conformance_suite(tool: &dyn Tool) {
    test_name_is_nonempty(tool);
    test_name_has_no_spaces(tool);
    test_description_is_nonempty(tool);
    test_parameters_is_valid_json_schema(tool);
    test_call_with_empty_string(tool).await;
}

fn test_name_is_nonempty(tool: &dyn Tool) {
    assert!(!tool.name().is_empty(), "tool name must not be empty");
}

fn test_name_has_no_spaces(tool: &dyn Tool) {
    assert!(
        !tool.name().contains(' '),
        "tool name '{}' must not contain spaces — use underscores",
        tool.name()
    );
}

fn test_description_is_nonempty(tool: &dyn Tool) {
    assert!(
        !tool.description().is_empty(),
        "tool description must not be empty"
    );
    assert!(
        tool.description().len() >= 10,
        "tool description is too short to be useful"
    );
}

fn test_parameters_is_valid_json_schema(tool: &dyn Tool) {
    let params = tool.parameters();
    assert_eq!(
        params["type"].as_str(),
        Some("object"),
        "parameters root must be type:object"
    );
    assert!(
        params["properties"].is_object(),
        "parameters must have a 'properties' object"
    );
}

async fn test_call_with_empty_string(tool: &dyn Tool) {
    // Tools must not panic on empty input — they may return Err
    let _ = tool.call("").await;
}
```

### Using the Suite

```rust
#[tokio::test]
async fn word_count_tool_conforms() {
    conformance_suite(&WordCount).await;
}
```

</conformance-suites>

<chain-conformance>
## Chain Conformance

```rust
// tests/conformance/chain.rs
use langchain_rust::chain::Chain;
use crate::test_utils::FakeLLM;

pub async fn conformance_suite(chain: &dyn Chain) {
    test_output_keys_nonempty(chain);
    test_get_input_keys_returns_vec(chain);
}

fn test_output_keys_nonempty(chain: &dyn Chain) {
    let keys = chain.get_output_keys();
    assert!(!keys.is_empty(), "chain must declare at least one output key");
}

fn test_get_input_keys_returns_vec(chain: &dyn Chain) {
    let _ = chain.get_input_keys(); // must not panic
}
```

</chain-conformance>

<proptest>
## Property-Based Tests (proptest)

Add to `Cargo.toml`:

```toml
[dev-dependencies]
proptest = "1"
```

### Prompt template roundtrip invariant

```rust
// tests/proptest/prompt.rs
use proptest::prelude::*;
use langchain_rust::{prompt_args, template_fstring};

proptest! {
    #[test]
    fn prompt_args_stores_all_keys(
        key in "[a-z][a-z0-9_]{0,15}",
        value in ".*"
    ) {
        let args = prompt_args! { &key => &value };
        prop_assert!(args.contains_key(&key));
    }

    #[test]
    fn window_buffer_never_exceeds_capacity(
        capacity in 1usize..=20,
        messages in proptest::collection::vec(".*", 0..=50),
    ) {
        use langchain_rust::{memory::WindowBufferMemory, schemas::Message};
        let mut mem = WindowBufferMemory::new(capacity);
        for msg in &messages {
            mem.add_user_message(msg.clone());
        }
        prop_assert!(mem.messages().len() <= capacity * 2); // user+AI pairs
    }
}
```

</proptest>

<fuzzing>
## Fuzz Targets (cargo-fuzz)

Install: `cargo install cargo-fuzz`
Init: `cargo fuzz init` (creates `fuzz/` directory)

### Fuzz Tool::parse_input

```rust
// fuzz/fuzz_targets/tool_parse_input.rs
#![no_main]
use libfuzzer_sys::fuzz_target;
use langchain_rust::tools::Tool;

// A concrete tool to fuzz
struct EchoTool;
#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String { "echo".into() }
    fn description(&self) -> String { "Echoes input.".into() }
    async fn run(&self, input: serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
        Ok(input.to_string())
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let tool = EchoTool;
            let _ = tool.call(s).await; // must not panic
        });
    }
});
```

Run: `cargo fuzz run tool_parse_input`

### Fuzz prompt template formatting

```rust
// fuzz/fuzz_targets/prompt_format.rs
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        use langchain_rust::{prompt_args, template_fstring};
        use langchain_rust::prompt::{HumanMessagePromptTemplate, FormatPrompter};

        // Arbitrary template — format must not panic even on bad input
        let template = template_fstring!("{input}", "input");
        let tmpl = HumanMessagePromptTemplate::new(template);
        let args = prompt_args! { "input" => s };
        let _ = tmpl.format_prompt(args); // may return Err, must not panic
    }
});
```

</fuzzing>

<boundary-contracts>
## Boundary Contracts

Document explicit contracts for each trait. These are the invariants conformance tests enforce.

### Tool

- `name()` is non-empty, no spaces, stable across calls
- `description()` is non-empty, >= 10 chars
- `parameters()` root is `{ "type": "object", "properties": {...} }`
- `call("")` returns `Ok` or `Err` — never panics
- `call(any_valid_utf8)` — never panics

### Chain

- `get_output_keys()` returns at least one key
- `call({})` with missing keys returns `Err(ChainError::...)` — never panics
- `invoke` result equals `call` result's `.generation` field

### BaseMemory

- `messages()` after `clear()` returns empty vec
- `messages()` after N `add_user_message` calls has >= N messages
- `WindowBufferMemory(k)`: `messages().len()` never exceeds `2k` (user+AI pairs)

### LLM

- `generate(&[])` returns `Ok` or typed `Err` — never panics
- `invoke(str)` result equals `generate([HumanMessage(str)])` result's `.generation`
  </boundary-contracts>

<test-organization>
## Test Organization

```
tests/
  conformance/
    tool.rs         # Tool trait conformance suite
    chain.rs        # Chain trait conformance suite
    memory.rs       # BaseMemory trait conformance suite
    embedder.rs     # Embedder trait conformance suite
  proptest/
    prompt.rs       # PromptArgs, template formatting invariants
    memory.rs       # WindowBufferMemory capacity invariants
fuzz/
  fuzz_targets/
    tool_parse_input.rs
    prompt_format.rs
src/
  test_utils.rs     # FakeLLM, FailingLLM, test helpers
```

</test-organization>
