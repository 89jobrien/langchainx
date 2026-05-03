---
name: prompt-args
description: JOB-254 — PromptArgs pitfalls, required key validation, and typed input patterns.
---

<oneliner>
PromptArgs is HashMap<String, Value>. Missing keys cause runtime panics. Always match
prompt_args! keys exactly to template_fstring! variable names.
</oneliner>

<type-definition>
## Type Definition

```rust
// src/prompt/mod.rs
pub type PromptArgs = HashMap<String, serde_json::Value>;
```

It is an alias — not a newtype. There is no compile-time enforcement of required keys.
</type-definition>

<prompt-args-macro>
## prompt_args! Macro

```rust
use langchainx::prompt_args;

let args = prompt_args! {
    "input" => "Hello world",
    "context" => "Some context",
    "count" => 42,
};
// Expands to HashMap::from([("input".to_string(), json!("Hello world")), ...])
```

Values are converted via `serde_json::json!`. Any `Serialize` type works.
</prompt-args-macro>

<key-contracts>
## Chain Input Key Contracts

Each chain documents its required keys. These are checked only at runtime.

| Chain                          | Required keys                                 |
| ------------------------------ | --------------------------------------------- |
| `LLMChain`                     | whatever variables are in the prompt template |
| `ConversationalChain`          | `"input"`                                     |
| `ConversationalRetrieverChain` | `"input"`                                     |
| `AgentExecutor`                | `"input"` (hardcoded)                         |

Use `chain.get_input_keys()` to inspect at runtime:

```rust
let keys = chain.get_input_keys();
assert!(keys.contains(&"input".to_string()));
```

</key-contracts>

<validation-pattern>
## Validating Keys Before Calling (current best practice)

Until JOB-254 is resolved, validate manually:

```rust
fn validate_args(chain: &dyn Chain, args: &PromptArgs) -> Result<(), ChainError> {
    for key in chain.get_input_keys() {
        if !args.contains_key(&key) {
            return Err(ChainError::MissingObject(
                format!("missing required input key: '{key}'")
            ));
        }
    }
    Ok(())
}
```

</validation-pattern>

<fix-key-mismatch>
## Common Mistake: Key Mismatch

```rust
// Template uses "question" but args use "input"
let prompt = message_formatter![fmt_template!(
    HumanMessagePromptTemplate::new(template_fstring!("{question}", "question"))
)];

// WRONG
let args = prompt_args! { "input" => "What is Rust?" };
// chain.invoke(args) → runtime error: missing variable "question"

// CORRECT
let args = prompt_args! { "question" => "What is Rust?" };
```

</fix-key-mismatch>
