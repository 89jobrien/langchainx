---
name: error-handling
description: ToolError on Tool::run and Tool::call, thiserror patterns, and existing error types per module.
---

<oneliner>
Tool::run and Tool::call return ToolError (not Box<dyn Error>). Use thiserror for
typed internal errors and convert via ToolError::Other for external error types.
</oneliner>

<current-state>
## Current State

```rust
// Tool trait — actual signatures
async fn run(&self, input: Value) -> Result<String, ToolError>;
async fn call(&self, input: &str) -> Result<String, ToolError>;
```

`AgentExecutor` receives the error and stringifies it for the agent:

```rust
Err(err) => format!("The tool return the following error: {}", err)
```

</current-state>

<toolerror>
## ToolError

```rust
// src/tools/error.rs
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}
```

Use `ToolError::InvalidInput` for bad user input, `ToolError::ExecutionFailed` for
runtime failures, and `ToolError::Other` to wrap external errors via `?`.
</toolerror>

<correct-pattern>
## Correct Pattern: Typed Errors in Implementations

```rust
use langchainx::tools::{Tool, ToolError};
use serde_json::Value;

pub struct MyTool;

#[async_trait::async_trait]
impl Tool for MyTool {
    fn name(&self) -> String { "my_tool".to_string() }
    fn description(&self) -> String { "Does something useful.".to_string() }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let query = input.as_str()
            .ok_or_else(|| ToolError::InvalidInput("expected string".into()))?;

        let result = call_api(query).await
            .map_err(|e| ToolError::Other(Box::new(e)))?;

        Ok(result)
    }
}
```

For external errors that implement `std::error::Error + Send + Sync`, the `From` impl
on `ToolError::Other` allows `?` to work directly.
</correct-pattern>

<existing-error-types>
## Existing Error Types

Each module has a typed error via `thiserror`:

| Module           | Error type      |
| ---------------- | --------------- |
| LLM              | `LLMError`      |
| Chain            | `ChainError`    |
| Agent            | `AgentError`    |
| Tool             | `ToolError`     |
| Embedding        | `EmbedderError` |
| Prompt           | `PromptError`   |
| Document loaders | `LoaderError`   |

Use these when implementing new code in those modules. Do not introduce `Box<dyn Error>`
return types anywhere.
</existing-error-types>
