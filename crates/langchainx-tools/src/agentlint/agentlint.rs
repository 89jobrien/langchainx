//! `Tool` adapter for validating agent configuration files with `agentlint`.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::{Tool, ToolError};

/// Tool adapter for `agentlint` — validates AI agent harness files.
///
/// Checks CLAUDE.md, .cursor/rules, AGENTS.md, hooks, skills, and settings
/// for structural and semantic violations.
pub struct AgentlintTool {
    base_dir: PathBuf,
}

impl AgentlintTool {
    /// Creates a tool rooted at the supplied project directory.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AgentlintInput {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    format: Option<String>,
}

#[async_trait]
impl Tool for AgentlintTool {
    fn name(&self) -> String {
        "Agentlint".into()
    }

    fn description(&self) -> String {
        "Run agentlint to validate AI agent configuration files \
         (CLAUDE.md, .cursor/rules, AGENTS.md, hooks, skills, settings). \
         Returns diagnostics in GNU or JSON format."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Directory to lint (defaults to project root)."
                },
                "format": {
                    "type": "string",
                    "description": "Output format: 'text' (default) or 'json'.",
                    "enum": ["text", "json"]
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
        let parsed: AgentlintInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let target = match parsed.path {
            Some(ref p) => self.base_dir.join(p),
            None => self.base_dir.clone(),
        };

        let mut cmd = Command::new("agentlint");
        cmd.arg(&target);

        if let Some(ref fmt) = parsed.format {
            cmd.arg("--format").arg(fmt);
        }

        let output = cmd
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run agentlint: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if stdout.is_empty() && output.status.success() {
            Ok("No agentlint violations found.".into())
        } else {
            Ok(stdout)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_schema_valid() {
        let tool = AgentlintTool::new(".");
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
    }
}
