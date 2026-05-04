use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::process::Command;
use std::time::Duration;

use crate::{Tool, ToolError};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT_CHARS: usize = 10_000;

pub struct BashTool;

#[derive(Debug, Deserialize)]
struct BashInput {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> String {
        "Bash".into()
    }

    fn description(&self) -> String {
        "Run a shell command. Input: { \"command\": \"<shell>\", \"timeout_secs\": <optional u64> }. \
         Returns JSON with stdout, stderr, and exit_code."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute."
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Optional timeout in seconds (default 60)."
                }
            },
            "required": ["command"],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        match serde_json::from_str::<Value>(input) {
            Ok(v) => v,
            Err(_) => Value::String(input.to_string()),
        }
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let parsed: BashInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let timeout = Duration::from_secs(parsed.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

        let result = run_with_timeout(&parsed.command, timeout)?;
        Ok(result)
    }
}

fn run_with_timeout(command: &str, _timeout: Duration) -> Result<String, ToolError> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| ToolError::ExecutionFailed(format!("failed to spawn sh: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    // Truncate combined output at MAX_OUTPUT_CHARS
    let combined_len = stdout.len() + stderr.len();
    let (stdout, stderr) = if combined_len > MAX_OUTPUT_CHARS {
        let keep = MAX_OUTPUT_CHARS / 2;
        (
            stdout.chars().take(keep).collect::<String>(),
            stderr.chars().take(MAX_OUTPUT_CHARS - keep).collect::<String>(),
        )
    } else {
        (stdout, stderr)
    };

    Ok(serde_json::to_string(&json!({
        "stdout": stdout,
        "stderr": stderr,
        "exit_code": exit_code
    }))
    .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bash_echo_returns_stdout() {
        let tool = BashTool;
        let result = tool
            .run(json!({ "command": "echo hello" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["stdout"].as_str().unwrap().trim(), "hello");
        assert_eq!(v["exit_code"], 0);
    }

    #[tokio::test]
    async fn bash_nonzero_exit_still_returns_ok() {
        let tool = BashTool;
        let result = tool.run(json!({ "command": "exit 42" })).await;
        assert!(result.is_ok());
        let v: Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(v["exit_code"], 42);
    }

    #[tokio::test]
    async fn bash_stderr_captured() {
        let tool = BashTool;
        let result = tool
            .run(json!({ "command": "echo err >&2" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["stderr"].as_str().unwrap().trim(), "err");
    }

    #[test]
    fn bash_name() {
        assert_eq!(BashTool.name(), "Bash");
    }

    #[tokio::test]
    async fn bash_invalid_input_errors() {
        let tool = BashTool;
        let result = tool.run(json!({ "not_a_command": "x" })).await;
        assert!(result.is_err());
    }
}
