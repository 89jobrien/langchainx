//! [`ContainerTool`] — langchainx `Tool` adapter for any [`ContainerRuntime`].

use serde_json::{Value, json};

use super::{ContainerRuntime, RunConfig, SandboxConfig, detect_runtime};
use crate::{Tool, ToolError};

/// A langchainx tool that delegates container operations to any
/// [`ContainerRuntime`] backend.
///
/// # Example
/// ```rust,ignore
/// // Auto-detect the best runtime
/// let tool = ContainerTool::auto().expect("no runtime found");
///
/// // Or use a specific runtime
/// let tool = ContainerTool::new(Box::new(DockerRuntime::new("docker")));
/// ```
pub struct ContainerTool {
    runtime: Box<dyn ContainerRuntime>,
}

impl ContainerTool {
    /// Creates a tool backed by the supplied container runtime.
    pub fn new(runtime: Box<dyn ContainerRuntime>) -> Self {
        Self { runtime }
    }

    /// Auto-detect the best available runtime and wrap it.
    pub fn auto() -> Option<Self> {
        detect_runtime().map(|rt| Self { runtime: rt })
    }

    /// The underlying runtime name.
    pub fn runtime_name(&self) -> &str {
        self.runtime.name()
    }
}

impl Tool for ContainerTool {
    fn name(&self) -> String {
        "Container".into()
    }

    fn description(&self) -> String {
        format!(
            "Manage containers via {} runtime. Actions: ps, pull, run, stop, \
             pause, resume, rm, exec, logs, sandbox, prune, rmi, snapshot.",
            self.runtime.name()
        )
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": [
                        "ps", "pull", "run", "stop", "pause", "resume", "rm",
                        "exec", "logs", "sandbox", "prune", "rmi", "snapshot"
                    ]
                },
                "sub_action": {
                    "type": "string",
                    "enum": ["save", "restore", "list"],
                    "description": "Sub-action for snapshot command."
                },
                "image": { "type": "string" },
                "tag": { "type": "string", "default": "latest" },
                "command": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "id": { "type": "string" },
                "container_id": { "type": "string" },
                "cmd": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "all": { "type": "boolean" },
                "memory": { "type": "integer" },
                "cpu_weight": { "type": "integer" },
                "network": { "type": "string" },
                "privileged": { "type": "boolean" },
                "volumes": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "env": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "name": { "type": "string" },
                "platform": { "type": "string" },
                "rm": { "type": "boolean" },
                "script": { "type": "string" },
                "memory_mb": { "type": "integer" },
                "timeout": { "type": "integer" },
                "dry_run": { "type": "boolean" },
                "image_ref": { "type": "string" }
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

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let action = input["action"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("missing 'action' field".into()))?;

        match action {
            "ps" => {
                let containers = self.runtime.ps().await?;
                Ok(serde_json::to_string_pretty(&containers).unwrap_or_else(|_| "[]".into()))
            }

            "pull" => {
                let image = str_field(&input, "image")?;
                let tag = input["tag"].as_str().unwrap_or("latest");
                self.runtime.pull(image, tag).await?;
                Ok(format!("Pulled {image}:{tag}"))
            }

            "run" => {
                let config = parse_run_config(&input)?;
                let id = self.runtime.run(&config).await?;
                Ok(id)
            }

            "stop" => {
                let id = str_field(&input, "id")?;
                self.runtime.stop(id).await?;
                Ok(format!("Stopped {id}"))
            }

            "pause" => {
                let id = str_field(&input, "id")?;
                self.runtime.pause(id).await?;
                Ok(format!("Paused {id}"))
            }

            "resume" => {
                let id = str_field(&input, "id")?;
                self.runtime.resume(id).await?;
                Ok(format!("Resumed {id}"))
            }

            "rm" => {
                let all = input["all"].as_bool().unwrap_or(false);
                if all {
                    self.runtime.rm_all().await?;
                    Ok("Removed all stopped containers".into())
                } else {
                    let id = str_field(&input, "id")?;
                    self.runtime.rm(id).await?;
                    Ok(format!("Removed {id}"))
                }
            }

            "exec" => {
                let id = str_field(&input, "container_id")?;
                let cmd = str_array(&input, "cmd")?;
                if cmd.is_empty() {
                    return Err(ToolError::InvalidInput(
                        "exec requires a non-empty cmd".into(),
                    ));
                }
                let refs: Vec<&str> = cmd.iter().map(String::as_str).collect();
                self.runtime.exec(id, &refs).await
            }

            "logs" => {
                let id = str_field(&input, "id")?;
                self.runtime.logs(id).await
            }

            "prune" => {
                let dry_run = input["dry_run"].as_bool().unwrap_or(false);
                self.runtime.prune(dry_run).await
            }

            "rmi" => {
                let image_ref = str_field(&input, "image_ref")?;
                self.runtime.rmi(image_ref).await?;
                Ok(format!("Removed image {image_ref}"))
            }

            "sandbox" => {
                let config = parse_sandbox_config(&input)?;
                self.runtime.sandbox(&config).await
            }

            "snapshot" => {
                let sub = str_field(&input, "sub_action")?;
                let cid = str_field(&input, "container_id")?;
                match sub {
                    "save" => {
                        let name = input["name"].as_str();
                        self.runtime.snapshot_save(cid, name).await
                    }
                    "restore" => {
                        let name = str_field(&input, "name")?;
                        self.runtime.snapshot_restore(cid, name).await
                    }
                    "list" => {
                        let snaps = self.runtime.snapshot_list(cid).await?;
                        Ok(serde_json::to_string_pretty(&snaps).unwrap_or_else(|_| "[]".into()))
                    }
                    other => Err(ToolError::InvalidInput(format!(
                        "unknown snapshot sub_action: {other}"
                    ))),
                }
            }

            other => Err(ToolError::InvalidInput(format!("unknown action: {other}"))),
        }
    }
}

fn str_field<'a>(input: &'a Value, field: &str) -> Result<&'a str, ToolError> {
    input[field]
        .as_str()
        .ok_or_else(|| ToolError::InvalidInput(format!("missing '{field}' field")))
}

fn str_array(input: &Value, field: &str) -> Result<Vec<String>, ToolError> {
    match input.get(field) {
        Some(Value::Array(arr)) => Ok(arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()),
        _ => Ok(Vec::new()),
    }
}

fn parse_run_config(input: &Value) -> Result<RunConfig, ToolError> {
    Ok(RunConfig {
        image: str_field(input, "image")?.to_string(),
        tag: input["tag"].as_str().unwrap_or("latest").to_string(),
        command: str_array(input, "command")?,
        name: input["name"].as_str().map(String::from),
        env: str_array(input, "env")?,
        volumes: str_array(input, "volumes")?,
        memory: input["memory"].as_u64(),
        cpu_weight: input["cpu_weight"].as_u64(),
        network: input["network"].as_str().map(String::from),
        privileged: input["privileged"].as_bool().unwrap_or(false),
        platform: input["platform"].as_str().map(String::from),
        auto_remove: input["rm"].as_bool().unwrap_or(false),
    })
}

fn parse_sandbox_config(input: &Value) -> Result<SandboxConfig, ToolError> {
    Ok(SandboxConfig {
        script: str_field(input, "script")?.to_string(),
        image: input["image"]
            .as_str()
            .unwrap_or("minibox-sandbox")
            .to_string(),
        tag: input["tag"].as_str().unwrap_or("latest").to_string(),
        memory_mb: input["memory_mb"]
            .as_u64()
            .unwrap_or(super::DEFAULT_SANDBOX_MEMORY_MB),
        timeout_secs: input["timeout"]
            .as_u64()
            .unwrap_or(super::DEFAULT_SANDBOX_TIMEOUT_SECS),
        volumes: str_array(input, "volumes")?,
        network: input["network"].as_bool().unwrap_or(false),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_tool_auto_returns_option() {
        // May be Some or None depending on environment
        let _ = ContainerTool::auto();
    }

    #[test]
    fn parse_run_config_minimal() {
        let input = json!({"action": "run", "image": "alpine"});
        let config = parse_run_config(&input).unwrap();
        assert_eq!(config.image, "alpine");
        assert_eq!(config.tag, "latest");
        assert!(!config.privileged);
    }

    #[test]
    fn parse_run_config_full() {
        let input = json!({
            "action": "run",
            "image": "ubuntu",
            "tag": "22.04",
            "command": ["sleep", "300"],
            "name": "test",
            "env": ["FOO=bar"],
            "volumes": ["/tmp:/tmp"],
            "memory": 1073741824_u64,
            "network": "bridge",
            "privileged": true,
            "rm": true
        });
        let config = parse_run_config(&input).unwrap();
        assert_eq!(config.image, "ubuntu");
        assert_eq!(config.tag, "22.04");
        assert_eq!(config.command, vec!["sleep", "300"]);
        assert!(config.privileged);
        assert!(config.auto_remove);
        assert_eq!(config.memory, Some(1073741824));
    }

    #[test]
    fn parse_sandbox_config_defaults() {
        let input = json!({"action": "sandbox", "script": "/tmp/test.py"});
        let config = parse_sandbox_config(&input).unwrap();
        assert_eq!(config.script, "/tmp/test.py");
        assert_eq!(config.memory_mb, 512);
        assert_eq!(config.timeout_secs, 60);
    }

    #[test]
    fn str_field_returns_error_on_missing() {
        let input = json!({"action": "run"});
        assert!(str_field(&input, "image").is_err());
    }

    #[test]
    fn str_array_returns_empty_on_missing() {
        let input = json!({"action": "run"});
        assert!(str_array(&input, "cmd").unwrap().is_empty());
    }
}
