---
name: tools
description: Implementing the Tool trait, defining parameters JSON schema, and registering tools with AgentExecutor.
---

<oneliner>
Implement Tool via #[async_trait]. Required methods: name(), description(), run(Value).
Optional: parameters() for OpenAI function-call schema. parse_input() has a sensible default.
</oneliner>

<tool-trait>
## Tool Trait

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use langchain_rust::tools::Tool;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;

    // OpenAI function-call JSON schema — override for structured inputs
    fn parameters(&self) -> Value;  // default: { type: object, properties: { input: string } }

    // Called by AgentExecutor
    async fn call(&self, input: &str) -> Result<String, Box<dyn Error>>;

    // You implement this
    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>>;

    // Parses raw string input → Value. Default handles JSON or plain string.
    async fn parse_input(&self, input: &str) -> Value;
}
```

</tool-trait>

<basic-tool>
## Basic Tool Implementation

```rust
use async_trait::async_trait;
use serde_json::Value;
use std::error::Error;
use langchain_rust::tools::Tool;

pub struct WordCount;

#[async_trait]
impl Tool for WordCount {
    fn name(&self) -> String {
        "word_count".to_string()
    }

    fn description(&self) -> String {
        "Counts the number of words in a text string. \
         Use when you need to know how many words are in a passage."
            .to_string()
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let text = input
            .as_str()
            .ok_or("input must be a string")?;
        Ok(format!("{} words", text.split_whitespace().count()))
    }
}
```

</basic-tool>

<structured-tool>
## Tool with Structured Input (OpenAI function calling)

Override `parameters()` to define the JSON schema, then extract fields in `run()`.

```rust
use serde_json::{json, Value};

pub struct Calculator;

#[async_trait]
impl Tool for Calculator {
    fn name(&self) -> String { "calculator".to_string() }

    fn description(&self) -> String {
        "Evaluates a math expression. Use for arithmetic.".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "A math expression, e.g. '2 + 2' or '100 / 4'"
                }
            },
            "required": ["expression"]
        })
    }

    async fn run(&self, input: Value) -> Result<String, Box<dyn Error>> {
        let expr = input["expression"]
            .as_str()
            .ok_or("missing 'expression' field")?;
        // evaluate expr...
        Ok("42".to_string())
    }
}
```

</structured-tool>

<register-tools>
## Registering with AgentExecutor

Tools are passed as `Vec<Arc<dyn Tool>>` via the agent builder, not directly to AgentExecutor.

```rust
use std::sync::Arc;
use langchain_rust::agent::{AgentExecutor, OpenAIToolsAgentBuilder};
use langchain_rust::tools::Tool;

let tools: Vec<Arc<dyn Tool>> = vec![
    Arc::new(WordCount),
    Arc::new(Calculator),
];

let agent = OpenAIToolsAgentBuilder::new()
    .tools(&tools)
    .llm(OpenAI::default())
    .build()?;

let executor = AgentExecutor::from_agent(agent)
    .with_max_iterations(10);
```

</register-tools>

<fix-tool-name-spaces>
## Common Mistake: Tool names with spaces

`AgentExecutor` normalizes tool names by replacing spaces with underscores. Keep names
lowercase with underscores to avoid mismatches.

```rust
// WRONG — spaces cause lookup failures
fn name(&self) -> String { "Word Count Tool".to_string() }

// CORRECT
fn name(&self) -> String { "word_count".to_string() }
```

</fix-tool-name-spaces>
