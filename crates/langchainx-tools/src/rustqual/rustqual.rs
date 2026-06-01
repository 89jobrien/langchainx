use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::{Tool, ToolError};

/// Tool adapter for the `rustqual` code quality analyzer.
///
/// Runs rustqual on a target path and returns structured JSON findings
/// covering six dimensions: IOSP, Complexity, DRY, SRP, Coupling, Test Quality.
pub struct RustqualTool {
    base_dir: PathBuf,
}

impl RustqualTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RustqualInput {
    path: String,
    #[serde(default)]
    strict_closures: bool,
    #[serde(default)]
    strict_iterators: bool,
    #[serde(default)]
    verbose: bool,
}

#[async_trait]
impl Tool for RustqualTool {
    fn name(&self) -> String {
        "Rustqual".into()
    }

    fn description(&self) -> String {
        "Run rustqual code quality analysis on a Rust file or directory. \
         Returns JSON findings across six quality dimensions: \
         IOSP, Complexity, DRY, SRP, Coupling, Test Quality."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "File or directory to analyze (relative to project root)."
                },
                "strict_closures": {
                    "type": "boolean",
                    "description": "Treat closures as logic (stricter analysis).",
                    "default": false
                },
                "strict_iterators": {
                    "type": "boolean",
                    "description": "Treat iterator chains as logic.",
                    "default": false
                },
                "verbose": {
                    "type": "boolean",
                    "description": "Show all functions, not just findings.",
                    "default": false
                }
            },
            "required": ["path"],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_string()))
    }

    // qual:allow(iosp) reason: "tool I/O boundary"
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let parsed: RustqualInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let target = crate::shell::validate_path(&self.base_dir, &parsed.path)?;
        let mut cmd = Command::new("rustqual");
        cmd.arg("--json");
        cmd.arg(&target);

        if parsed.strict_closures {
            cmd.arg("--strict-closures");
        }
        if parsed.strict_iterators {
            cmd.arg("--strict-iterators");
        }
        if parsed.verbose {
            cmd.arg("--verbose");
        }

        let output = cmd
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run rustqual: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !stderr.is_empty() && stdout.is_empty() {
            return Err(ToolError::ExecutionFailed(stderr));
        }

        Ok(stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_schema_valid() {
        let tool = RustqualTool::new(".");
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["path"].is_object());
    }
}
