//! Traits and errors for tools that agents can call.
use async_trait::async_trait;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
/// Errors produced while parsing input or executing a tool.
pub enum ToolError {
    #[error("invalid input: {0}")]
    /// The supplied input does not satisfy the tool's requirements.
    InvalidInput(String),
    #[error("execution failed: {0}")]
    /// The tool could not complete its operation.
    ExecutionFailed(String),
    #[error(transparent)]
    /// An error from the tool's underlying implementation.
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait]
/// Defines a callable tool that agents can inspect and execute.
pub trait Tool: Send + Sync {
    /// Returns the name exposed to agents.
    fn name(&self) -> String;
    /// Describes what the tool does and when to use it.
    fn description(&self) -> String;

    /// Returns the JSON Schema for arguments accepted by the tool.
    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": self.description()
                }
            },
            "required": ["input"]
        })
    }

    /// Parses a string input and executes the tool.
    async fn call(&self, input: &str) -> Result<String, ToolError> {
        let input = self.parse_input(input).await;
        self.run(input).await
    }

    /// Executes the tool with parsed JSON input.
    async fn run(&self, input: Value) -> Result<String, ToolError>;

    /// Converts a string input into the JSON value accepted by [`Tool::run`].
    async fn parse_input(&self, input: &str) -> Value {
        log::info!("Using default implementation: {}", input);
        match serde_json::from_str::<Value>(input) {
            Ok(input) => {
                if let Some(s) = input["input"].as_str() {
                    Value::String(s.to_string())
                } else {
                    Value::String(input.to_string())
                }
            }
            Err(_) => Value::String(input.to_string()),
        }
    }
}
