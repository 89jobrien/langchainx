---
name: error-handling
description: JOB-255 — replacing Box<dyn Error> on Tool::run with typed ToolError, thiserror patterns.
---

<oneliner>
Tool::run currently returns Box<dyn Error> — opaque, not matchable. New tools should
internally use typed errors and convert at the boundary. When JOB-255 lands, Tool::run
returns ToolError.
</oneliner>

<current-state>
## Current State

```rust
// Tool trait today
async fn run(&self, input: Value) -> Result<String, Box<dyn Error>>;
async fn call(&self, input: &str) -> Result<String, Box<dyn Error>>;
```

`AgentExecutor` receives the error and stringifies it:

```rust
Err(err) => format!("The tool return the following error: {}", err)
```

All error type information is lost. The agent sees a plain string.
</current-state>

<correct-pattern>
## Correct Pattern: Typed Errors in Implementations

Even while `Box<dyn Error>` is the return type, define internal typed errors and convert:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
enum MyToolError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),
    #[error("parse error: {0}")]
    ParseError(String),
}

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    async fn run(&self, input: Value) -> Result<String, Box<dyn std::error::Error>> {
        let query = input.as_str()
            .ok_or_else(|| MyToolError::InvalidInput("expected string".into()))?;

        let result = call_api(query).await
            .map_err(MyToolError::RequestFailed)?;

        Ok(result)
    }
    // ...
}
```

</correct-pattern>

<future-toolerror>
## Future ToolError (JOB-255 target)

```rust
// Will replace Box<dyn Error> once JOB-255 is implemented
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

Migration: change `run()` and `call()` signatures. Update `AgentExecutor` to pattern-match
`ToolError` variants and decide whether to break or continue.
</future-toolerror>

<existing-error-types>
## Existing Error Types

Each module already has a typed error via `thiserror`:

| Module           | Error type      |
| ---------------- | --------------- |
| LLM              | `LLMError`      |
| Chain            | `ChainError`    |
| Agent            | `AgentError`    |
| Embedding        | `EmbedderError` |
| Prompt           | `PromptError`   |
| Document loaders | `LoaderError`   |

Use these when implementing new code in those modules. Do not introduce new `Box<dyn Error>`
return types in non-Tool code.
</existing-error-types>
