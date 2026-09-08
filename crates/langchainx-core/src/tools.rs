//! Traits and errors for tools that agents can call.
use std::{future::Future, pin::Pin};

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
    fn call(&self, input: &str) -> impl Future<Output = Result<String, ToolError>> + Send {
        async move {
            let input = self.parse_input(input).await;
            self.run(input).await
        }
    }

    /// Executes the tool with parsed JSON input.
    fn run(&self, input: Value) -> impl Future<Output = Result<String, ToolError>> + Send;

    /// Converts a string input into the JSON value accepted by [`Tool::run`].
    fn parse_input(&self, input: &str) -> impl Future<Output = Value> + Send {
        async move {
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
}

/// A boxed, sendable future used at dynamic tool boundaries.
pub type BoxToolFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Object-safe adapter for dynamically dispatched [`Tool`] implementations.
pub trait DynTool: Send + Sync {
    /// Returns the wrapped tool name.
    fn dyn_name(&self) -> String;
    /// Returns the wrapped tool description.
    fn dyn_description(&self) -> String;
    /// Returns the wrapped tool argument schema.
    fn dyn_parameters(&self) -> Value;
    /// Parses and executes the wrapped tool through a boxed future.
    fn dyn_call<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Result<String, ToolError>>;
    /// Executes the wrapped tool through a boxed future.
    fn dyn_run(&self, input: Value) -> BoxToolFuture<'_, Result<String, ToolError>>;
    /// Parses input through a boxed future.
    fn dyn_parse_input<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Value>;
}

impl<T: Tool> DynTool for T {
    fn dyn_name(&self) -> String {
        Tool::name(self)
    }

    fn dyn_description(&self) -> String {
        Tool::description(self)
    }

    fn dyn_parameters(&self) -> Value {
        Tool::parameters(self)
    }

    fn dyn_call<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Result<String, ToolError>> {
        Box::pin(Tool::call(self, input))
    }

    fn dyn_run(&self, input: Value) -> BoxToolFuture<'_, Result<String, ToolError>> {
        Box::pin(Tool::run(self, input))
    }

    fn dyn_parse_input<'a>(&'a self, input: &'a str) -> BoxToolFuture<'a, Value> {
        Box::pin(Tool::parse_input(self, input))
    }
}
