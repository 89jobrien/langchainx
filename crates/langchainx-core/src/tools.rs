use async_trait::async_trait;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error(transparent)]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;

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

    async fn call(&self, input: &str) -> Result<String, ToolError> {
        let input = self.parse_input(input).await;
        self.run(input).await
    }

    async fn run(&self, input: Value) -> Result<String, ToolError>;

    // TODO(#35): add unit tests for parse_input -- covers JSON extraction
    //   fallback logic. Add fuzz target for adversarial input strings.
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
