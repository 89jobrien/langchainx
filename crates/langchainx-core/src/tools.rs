use std::{future::Future, pin::Pin};

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

    fn call(&self, input: &str) -> impl Future<Output = Result<String, ToolError>> + Send {
        async move {
            let input = self.parse_input(input).await;
            self.run(input).await
        }
    }

    fn run(&self, input: Value) -> impl Future<Output = Result<String, ToolError>> + Send;

    fn parse_input(&self, input: &str) -> impl Future<Output = Value> + Send {
        async move {
            log::info!("Using default implementation: {}", input);
            match serde_json::from_str::<Value>(input) {
                Ok(input) => {
                    if input["input"].is_string() {
                        Value::String(input["input"].as_str().unwrap().to_string())
                    } else {
                        Value::String(input.to_string())
                    }
                }
                Err(_) => Value::String(input.to_string()),
            }
        }
    }
}

pub type BoxToolFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DynTool: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn parameters(&self) -> Value;
    fn call<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Result<String, ToolError>>;
    fn run(&self, input: Value) -> BoxToolFuture<'_, Result<String, ToolError>>;
    fn parse_input<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Value>;
}

impl<T: Tool> DynTool for T {
    fn name(&self) -> String {
        Tool::name(self)
    }

    fn description(&self) -> String {
        Tool::description(self)
    }

    fn parameters(&self) -> Value {
        Tool::parameters(self)
    }

    fn call<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Result<String, ToolError>> {
        Box::pin(Tool::call(self, input))
    }

    fn run(&self, input: Value) -> BoxToolFuture<'_, Result<String, ToolError>> {
        Box::pin(Tool::run(self, input))
    }

    fn parse_input<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Value> {
        Box::pin(Tool::parse_input(self, input))
    }
}
