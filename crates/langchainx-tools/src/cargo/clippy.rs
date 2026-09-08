//! `Tool` adapter for collecting Clippy diagnostics.
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::{Tool, ToolError};

/// Tool adapter for `cargo clippy` — Rust's official lint tool.
///
/// Runs clippy with JSON message output for structured analysis.
pub struct ClippyTool {
    base_dir: PathBuf,
}

impl ClippyTool {
    /// Creates a Clippy tool rooted at the supplied project directory.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ClippyInput {
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    all_targets: bool,
    #[serde(default)]
    all_features: bool,
    #[serde(default)]
    deny_warnings: bool,
}

impl Tool for ClippyTool {
    fn name(&self) -> String {
        "Clippy".into()
    }

    fn description(&self) -> String {
        "Run cargo clippy on a Rust project. Returns JSON diagnostics \
         with lint warnings and errors including file locations and suggestions."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Specific package to lint (omit for entire workspace)."
                },
                "all_targets": {
                    "type": "boolean",
                    "description": "Lint all targets (tests, benches, examples).",
                    "default": false
                },
                "all_features": {
                    "type": "boolean",
                    "description": "Enable all feature flags.",
                    "default": false
                },
                "deny_warnings": {
                    "type": "boolean",
                    "description": "Treat warnings as errors.",
                    "default": false
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_string()))
    }

    // qual:allow(iosp) reason: "tool I/O boundary"
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let parsed: ClippyInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let mut cmd = Command::new("cargo");
        cmd.arg("clippy");
        cmd.arg("--message-format=json");
        cmd.current_dir(&self.base_dir);

        if let Some(ref pkg) = parsed.package {
            cmd.arg("-p").arg(pkg);
        }
        if parsed.all_targets {
            cmd.arg("--all-targets");
        }
        if parsed.all_features {
            cmd.arg("--all-features");
        }
        if parsed.deny_warnings {
            cmd.arg("--").arg("-D").arg("warnings");
        }

        let output = cmd
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run clippy: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        // Filter to only compiler-message lines (JSON diagnostics)
        let diagnostics: Vec<&str> = stdout
            .lines()
            .filter(|line| line.contains("\"reason\":\"compiler-message\""))
            .collect();

        if diagnostics.is_empty() && output.status.success() {
            Ok("No clippy warnings or errors.".into())
        } else {
            Ok(diagnostics.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_schema_valid() {
        let tool = ClippyTool::new(".");
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
    }
}
