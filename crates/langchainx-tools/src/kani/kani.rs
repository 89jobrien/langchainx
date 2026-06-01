use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::{Tool, ToolError};

/// Tool adapter for `kani` — the Rust formal verification tool.
///
/// Runs Kani model checking on a Rust source file and returns verification results.
pub struct KaniTool {
    base_dir: PathBuf,
}

impl KaniTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct KaniInput {
    file: String,
    #[serde(default)]
    harness: Option<String>,
    #[serde(default)]
    autoharness: bool,
    #[serde(default)]
    tests: bool,
}

#[async_trait]
impl Tool for KaniTool {
    fn name(&self) -> String {
        "Kani".into()
    }

    fn description(&self) -> String {
        "Run Kani formal verification on a Rust source file. \
         Can target a specific harness, use autoharness mode, \
         or verify test harnesses. Returns verification results."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "file": {
                    "type": "string",
                    "description": "Rust source file to verify (relative to project root)."
                },
                "harness": {
                    "type": "string",
                    "description": "Specific harness function to verify."
                },
                "autoharness": {
                    "type": "boolean",
                    "description": "Use autoharness mode to automatically create harnesses.",
                    "default": false
                },
                "tests": {
                    "type": "boolean",
                    "description": "Include test harnesses in verification.",
                    "default": false
                }
            },
            "required": ["file"],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_string()))
    }

    // qual:allow(iosp) reason: "tool I/O boundary"
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let parsed: KaniInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let target = crate::shell::validate_path(&self.base_dir, &parsed.file)?;

        let mut cmd = if parsed.autoharness {
            let mut c = Command::new("kani");
            c.arg("autoharness");
            c.arg(&target);
            c
        } else {
            let mut c = Command::new("kani");
            c.arg(&target);
            c
        };

        if let Some(ref harness) = parsed.harness {
            cmd.arg("--harness").arg(harness);
        }
        if parsed.tests {
            cmd.arg("--tests");
        }

        let output = cmd
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run kani: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Kani outputs results to stdout; stderr has progress info
        let mut result = stdout;
        if !output.status.success() {
            result.push_str("\n--- stderr ---\n");
            result.push_str(&stderr);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_schema_valid() {
        let tool = KaniTool::new(".");
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["file"].is_object());
    }
}
