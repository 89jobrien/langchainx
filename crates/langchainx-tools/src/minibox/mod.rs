//! Direct `mbx` CLI tool for minibox container operations.
mod input;

use input::{MiniboxInput, SnapshotAction};
use serde_json::{Value, json};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

use crate::{Tool, ToolError};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// A langchainx tool that wraps the `mbx` CLI for the minibox container runtime.
///
/// Supports: pull, run, ps, stop, pause, resume, rm, exec, logs, sandbox, prune, rmi, snapshot.
///
/// # Example
/// ```rust,ignore
/// let tool = MiniboxTool::new();
/// // or with custom binary path:
/// let tool = MiniboxTool::builder().mbx_path("/usr/local/bin/mbx").build();
/// ```
pub struct MiniboxTool {
    mbx_path: PathBuf,
    socket_path: Option<PathBuf>,
    timeout: Duration,
}

impl MiniboxTool {
    /// Creates a tool using `mbx` from `PATH` and the default timeout.
    pub fn new() -> Self {
        Self {
            mbx_path: PathBuf::from("mbx"),
            socket_path: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Returns a builder for the CLI path, socket, and timeout.
    pub fn builder() -> MiniboxToolBuilder {
        MiniboxToolBuilder::default()
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.mbx_path);
        if let Some(sock) = &self.socket_path {
            cmd.env("MINIBOX_SOCKET", sock);
        }
        cmd
    }

    // qual:allow(iosp) reason: "subprocess I/O boundary"
    async fn run_command(&self, args: &[&str]) -> Result<String, ToolError> {
        let mut cmd = self.build_command();
        cmd.args(args);

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .map_err(|_| {
                ToolError::ExecutionFailed(format!(
                    "mbx timed out after {}s",
                    self.timeout.as_secs()
                ))
            })?
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to spawn mbx: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            if stdout.is_empty() && !stderr.is_empty() {
                Ok(stderr)
            } else {
                Ok(stdout)
            }
        } else {
            let combined = if stderr.is_empty() { stdout } else { stderr };
            Err(ToolError::ExecutionFailed(format!(
                "mbx exited {}: {}",
                output.status,
                combined.trim()
            )))
        }
    }
}

impl Default for MiniboxTool {
    fn default() -> Self {
        Self::new()
    }
}

// -- Builder -----------------------------------------------------------------

#[derive(Default)]
/// Configures a [`MiniboxTool`].
pub struct MiniboxToolBuilder {
    mbx_path: Option<PathBuf>,
    socket_path: Option<PathBuf>,
    timeout: Option<Duration>,
}

impl MiniboxToolBuilder {
    /// Sets the `mbx` executable path.
    pub fn mbx_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.mbx_path = Some(path.into());
        self
    }

    /// Sets the minibox daemon socket through `MINIBOX_SOCKET`.
    pub fn socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    /// Sets the timeout for each CLI invocation.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the configured minibox tool.
    pub fn build(self) -> MiniboxTool {
        MiniboxTool {
            mbx_path: self.mbx_path.unwrap_or_else(|| PathBuf::from("mbx")),
            socket_path: self.socket_path,
            timeout: self.timeout.unwrap_or(DEFAULT_TIMEOUT),
        }
    }
}

// -- Tool impl ---------------------------------------------------------------

impl Tool for MiniboxTool {
    fn name(&self) -> String {
        "Minibox".into()
    }

    fn description(&self) -> String {
        "Interact with the minibox container runtime via the mbx CLI. \
         Supported actions: ps, pull, run, stop, pause, resume, rm, exec, logs, sandbox, prune, \
         rmi, snapshot."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "ps","pull","run","stop","pause","resume","rm","exec",
                        "logs","sandbox","prune","rmi","snapshot"
                    ],
                    "description": "The mbx subcommand to invoke."
                },
                "sub_action": {
                    "type": "string",
                    "enum": ["save","restore","list"],
                    "description": "Sub-action for the snapshot command."
                },
                "image": {
                    "type": "string",
                    "description": "Image name (used by pull, run, sandbox)."
                },
                "tag": {
                    "type": "string",
                    "description": "Image tag (default: latest)."
                },
                "command": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command and args to run inside the container (run action)."
                },
                "id": {
                    "type": "string",
                    "description": "Container ID or name (used by stop, pause, resume, rm, logs)."
                },
                "container_id": {
                    "type": "string",
                    "description": "Container ID or name (used by exec)."
                },
                "cmd": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Command to exec inside a running container."
                },
                "all": {
                    "type": "boolean",
                    "description": "Remove all stopped containers (rm action)."
                },
                "memory": {
                    "type": "integer",
                    "description": "Memory limit in bytes (run action)."
                },
                "cpu_weight": {
                    "type": "integer",
                    "description": "CPU weight 1-10000 (run action)."
                },
                "network": {
                    "type": "string",
                    "description": "Network mode: none, bridge, host, tailnet (run). Boolean for sandbox."
                },
                "privileged": {
                    "type": "boolean",
                    "description": "Grant full Linux capabilities (run action)."
                },
                "volumes": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Bind mounts in src:dst[:ro] format."
                },
                "env": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Environment variables as KEY=VALUE strings (run action)."
                },
                "name": {
                    "type": "string",
                    "description": "Human-readable container name (run action)."
                },
                "platform": {
                    "type": "string",
                    "description": "Target platform, e.g. linux/arm64 (pull, run)."
                },
                "rm": {
                    "type": "boolean",
                    "description": "Auto-remove container on exit (run action)."
                },
                "follow": {
                    "type": "boolean",
                    "description": "Stream new log output as it arrives (logs action)."
                },
                "script": {
                    "type": "string",
                    "description": "Path to script file on the host (sandbox action)."
                },
                "memory_mb": {
                    "type": "integer",
                    "description": "Memory limit in MB (sandbox action, default 512)."
                },
                "timeout": {
                    "type": "integer",
                    "description": "Timeout in seconds (sandbox action, default 60)."
                },
                "dry_run": {
                    "type": "boolean",
                    "description": "Show what would be done without acting (prune action)."
                },
                "image_ref": {
                    "type": "string",
                    "description": "Image reference in name:tag format (rmi action)."
                }
            },
            "required": ["action"],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        match serde_json::from_str::<Value>(input) {
            Ok(v) => v,
            Err(_) => Value::String(input.to_string()),
        }
    }

    // qual:allow(iosp) reason: "tool I/O boundary"
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let action: MiniboxInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        match action {
            MiniboxInput::Ps => self.run_command(&["ps"]).await,

            MiniboxInput::Pull {
                image,
                tag,
                platform,
            } => {
                let mut args = vec!["pull".to_string(), image, "--tag".to_string(), tag];
                if let Some(p) = platform {
                    args.push("--platform".to_string());
                    args.push(p);
                }
                self.run_command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                    .await
            }

            MiniboxInput::Run {
                image,
                tag,
                command,
                memory,
                cpu_weight,
                network,
                privileged,
                volumes,
                env,
                name,
                platform,
                rm,
            } => {
                let mut args = vec![
                    "run".to_string(),
                    "--tag".to_string(),
                    tag,
                    "--network".to_string(),
                    network,
                ];
                if let Some(m) = memory {
                    args.push("--memory".to_string());
                    args.push(m.to_string());
                }
                if let Some(w) = cpu_weight {
                    args.push("--cpu-weight".to_string());
                    args.push(w.to_string());
                }
                if privileged {
                    args.push("--privileged".to_string());
                }
                if rm {
                    args.push("--rm".to_string());
                }
                for v in &volumes {
                    args.push("-v".to_string());
                    args.push(v.clone());
                }
                for e in &env {
                    args.push("-e".to_string());
                    args.push(e.clone());
                }
                if let Some(n) = name {
                    args.push("--name".to_string());
                    args.push(n);
                }
                if let Some(p) = platform {
                    args.push("--platform".to_string());
                    args.push(p);
                }
                args.push(image);
                if !command.is_empty() {
                    args.push("--".to_string());
                    args.extend(command);
                }
                self.run_command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                    .await
            }

            MiniboxInput::Stop { id } => self.run_command(&["stop", &id]).await,

            MiniboxInput::Pause { id } => self.run_command(&["pause", &id]).await,

            MiniboxInput::Resume { id } => self.run_command(&["resume", &id]).await,

            MiniboxInput::Rm { id, all } => {
                if all {
                    self.run_command(&["rm", "--all"]).await
                } else if let Some(id) = id {
                    self.run_command(&["rm", &id]).await
                } else {
                    Err(ToolError::InvalidInput(
                        "rm requires either 'id' or 'all: true'".into(),
                    ))
                }
            }

            MiniboxInput::Exec { container_id, cmd } => {
                if cmd.is_empty() {
                    return Err(ToolError::InvalidInput(
                        "exec requires a non-empty cmd".into(),
                    ));
                }
                let mut args = vec!["exec".to_string(), container_id, "--".to_string()];
                args.extend(cmd);
                self.run_command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                    .await
            }

            MiniboxInput::Logs { id, follow } => {
                let mut args = vec!["logs".to_string(), id];
                if follow {
                    args.push("--follow".to_string());
                }
                self.run_command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                    .await
            }

            MiniboxInput::Sandbox {
                script,
                image,
                tag,
                memory_mb,
                timeout,
                volumes,
                network,
            } => {
                let mut args = vec![
                    "sandbox".to_string(),
                    script,
                    "--image".to_string(),
                    image,
                    "--tag".to_string(),
                    tag,
                    "--memory-mb".to_string(),
                    memory_mb.to_string(),
                    "--timeout".to_string(),
                    timeout.to_string(),
                ];
                for v in &volumes {
                    args.push("-v".to_string());
                    args.push(v.clone());
                }
                if network {
                    args.push("--network".to_string());
                }
                self.run_command(&args.iter().map(String::as_str).collect::<Vec<_>>())
                    .await
            }

            MiniboxInput::Prune { dry_run } => {
                if dry_run {
                    self.run_command(&["prune", "--dry-run"]).await
                } else {
                    self.run_command(&["prune"]).await
                }
            }

            MiniboxInput::Rmi { image_ref } => self.run_command(&["rmi", &image_ref]).await,

            MiniboxInput::Snapshot {
                sub_action,
                container_id,
                name,
            } => match sub_action {
                SnapshotAction::List => {
                    self.run_command(&["snapshot", "list", &container_id]).await
                }
                SnapshotAction::Save => {
                    let mut args = vec!["snapshot", "save", &container_id];
                    let name_str;
                    if let Some(ref n) = name {
                        name_str = n.clone();
                        args.push(&name_str);
                    }
                    self.run_command(&args).await
                }
                SnapshotAction::Restore => {
                    let n = name.ok_or_else(|| {
                        ToolError::InvalidInput("snapshot restore requires a snapshot name".into())
                    })?;
                    self.run_command(&["snapshot", "restore", &container_id, &n])
                        .await
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool() -> MiniboxTool {
        MiniboxTool::new()
    }

    #[test]
    fn builder_sets_path() {
        let t = MiniboxTool::builder()
            .mbx_path("/usr/local/bin/mbx")
            .build();
        assert_eq!(t.mbx_path, PathBuf::from("/usr/local/bin/mbx"));
    }

    #[test]
    fn builder_sets_socket() {
        let t = MiniboxTool::builder()
            .socket_path("/run/minibox/miniboxd.sock")
            .build();
        assert_eq!(
            t.socket_path,
            Some(PathBuf::from("/run/minibox/miniboxd.sock"))
        );
    }

    #[test]
    fn builder_sets_timeout() {
        let t = MiniboxTool::builder()
            .timeout(Duration::from_secs(10))
            .build();
        assert_eq!(t.timeout, Duration::from_secs(10));
    }

    #[test]
    fn rm_without_id_or_all_errors() {
        let tool = tool();
        let v = json!({ "action": "rm" });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.run(v));
        assert!(result.is_err());
    }

    #[test]
    fn exec_empty_cmd_errors() {
        let tool = tool();
        let v = json!({ "action": "exec", "container_id": "abc", "cmd": [] });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.run(v));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn tool_name_and_description() {
        let t = tool();
        assert_eq!(t.name(), "Minibox");
        assert!(!t.description().is_empty());
    }

    #[test]
    fn snapshot_restore_without_name_errors() {
        let tool = tool();
        let v = json!({
            "action": "snapshot",
            "sub_action": "restore",
            "container_id": "abc123"
        });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(tool.run(v));
        assert!(result.is_err());
    }
}
