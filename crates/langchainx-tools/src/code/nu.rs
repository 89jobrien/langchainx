use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crate::{Tool, ToolError};

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const MAX_OUTPUT_CHARS: usize = 10_000;

/// A langchainx tool that runs nushell commands and returns structured JSON output.
///
/// # Example
/// ```rust,ignore
/// let tool = NuTool::new();
/// let tool = NuTool::builder().timeout_secs(30).build();
/// ```
pub struct NuTool {
    nu_path: PathBuf,
    timeout_secs: u64,
}

impl NuTool {
    pub fn new() -> Self {
        Self {
            nu_path: PathBuf::from("nu"),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    pub fn builder() -> NuToolBuilder {
        NuToolBuilder::default()
    }

    /// Prepare the full command string, appending `| to json` if not already present.
    fn prepare_command(command: &str) -> String {
        let trimmed = command.trim_end();
        if trimmed.ends_with("| to json") {
            command.to_string()
        } else {
            format!("{} | to json", trimmed)
        }
    }

    fn run_nu(&self, command: &str) -> Result<String, ToolError> {
        let full_cmd = Self::prepare_command(command);
        let nu_path = self.nu_path.clone();
        let timeout = Duration::from_secs(self.timeout_secs);

        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            let result = std::process::Command::new(&nu_path)
                .args(["--no-config-file", "-c", &full_cmd])
                .output();
            let _ = tx.send(result);
        });

        let output = rx.recv_timeout(timeout).map_err(|_| {
            ToolError::ExecutionFailed("nu command timed out".to_string())
        })?.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("No such file") || msg.contains("not found") || msg.contains("os error 2") {
                ToolError::ExecutionFailed("nu not found on PATH".to_string())
            } else {
                ToolError::ExecutionFailed(format!("failed to spawn nu: {e}"))
            }
        })?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let truncated = if stdout.len() > MAX_OUTPUT_CHARS {
                stdout[..MAX_OUTPUT_CHARS].to_string()
            } else {
                stdout
            };
            Ok(truncated)
        } else {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let stderr_truncated = if stderr.len() > MAX_OUTPUT_CHARS {
                stderr[..MAX_OUTPUT_CHARS].to_string()
            } else {
                stderr
            };
            Ok(json!({
                "exit_code": exit_code,
                "stderr": stderr_truncated.trim()
            })
            .to_string())
        }
    }
}

impl Default for NuTool {
    fn default() -> Self {
        Self::new()
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct NuToolBuilder {
    nu_path: Option<PathBuf>,
    timeout_secs: Option<u64>,
}

impl NuToolBuilder {
    pub fn nu_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.nu_path = Some(path.into());
        self
    }

    pub fn timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub fn build(self) -> NuTool {
        NuTool {
            nu_path: self.nu_path.unwrap_or_else(|| PathBuf::from("nu")),
            timeout_secs: self.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS),
        }
    }
}

// ── Input ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct NuInput {
    command: String,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

// ── Tool impl ─────────────────────────────────────────────────────────────────

#[async_trait]
impl Tool for NuTool {
    fn name(&self) -> String {
        "NuTool".into()
    }

    fn description(&self) -> String {
        "Execute a nushell command and return structured JSON output. \
         Pass a 'command' field with a nu pipeline. An optional 'timeout_secs' \
         field overrides the default timeout."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "A nushell pipeline to execute (e.g. 'ls | where type == file | select name size')."
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
        let parsed: NuInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        // Allow per-call timeout override.
        let effective_timeout = parsed.timeout_secs.unwrap_or(self.timeout_secs);
        let tool = NuTool {
            nu_path: self.nu_path.clone(),
            timeout_secs: effective_timeout,
        };

        tool.run_nu(&parsed.command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_to_json_when_missing() {
        let cmd = "ls | where type == file | select name size";
        let prepared = NuTool::prepare_command(cmd);
        assert!(prepared.ends_with("| to json"), "got: {prepared}");
        assert_eq!(
            prepared,
            "ls | where type == file | select name size | to json"
        );
    }

    #[test]
    fn does_not_double_append_to_json() {
        let cmd = "ls | to json";
        let prepared = NuTool::prepare_command(cmd);
        assert_eq!(prepared, "ls | to json");
    }

    #[test]
    fn does_not_double_append_to_json_with_trailing_whitespace() {
        let cmd = "ls | to json   ";
        let prepared = NuTool::prepare_command(cmd);
        // trim_end before checking, so no duplication
        assert!(!prepared.contains("| to json | to json"));
    }

    #[test]
    fn nu_not_on_path_returns_tool_error() {
        let tool = NuTool::builder()
            .nu_path("/nonexistent/path/to/nu-binary-xyz")
            .build();
        let result = tool.run_nu("echo hello");
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ToolError::ExecutionFailed(msg) => {
                assert!(
                    msg.contains("nu not found") || msg.contains("failed to spawn"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ExecutionFailed, got: {other:?}"),
        }
    }

    /// Requires `nu` to be installed on PATH.
    #[tokio::test]
    #[ignore = "requires nu to be installed on PATH"]
    async fn nonzero_exit_returns_ok_with_exit_code() {
        let tool = NuTool::new();
        // `exit 1` in nu causes a non-zero exit
        let result = tool.run_nu("exit 1");
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        let json_str = result.unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(v["exit_code"].as_i64().unwrap() != 0);
    }

    /// Requires `nu` to be installed on PATH.
    #[tokio::test]
    #[ignore = "requires nu to be installed on PATH"]
    async fn nu_on_path_executes_command() {
        let tool = NuTool::new();
        let result = tool.run_nu("[1 2 3]");
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        let json_str = result.unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(v, serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn builder_sets_timeout() {
        let tool = NuTool::builder().timeout_secs(30).build();
        assert_eq!(tool.timeout_secs, 30);
    }

    #[test]
    fn builder_default_timeout() {
        let tool = NuTool::new();
        assert_eq!(tool.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[tokio::test]
    async fn tool_name_and_description() {
        let t = NuTool::new();
        assert_eq!(t.name(), "NuTool");
        assert!(!t.description().is_empty());
    }
}
