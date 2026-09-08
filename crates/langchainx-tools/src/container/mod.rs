//! Runtime-agnostic container abstraction for langchainx agents.
//!
//! Provides a [`ContainerRuntime`] trait with adapters for Docker, Podman,
//! and minibox. Use [`detect_runtime`] to auto-select the best available
//! runtime, or construct an adapter directly.

mod detect;
mod docker;
mod minibox;
mod tool;

pub use detect::detect_runtime;
pub use docker::DockerRuntime;
pub use minibox::MiniboxRuntime;
pub use tool::ContainerTool;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::ToolError;

/// Unique container identifier returned by [`ContainerRuntime::run`].
pub type ContainerId = String;

/// Container listing entry from [`ContainerRuntime::ps`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    /// Runtime-assigned container identifier.
    pub id: String,
    /// Optional human-readable container name.
    pub name: Option<String>,
    /// Image reference used by the container.
    pub image: String,
    /// Runtime-reported container status.
    pub status: String,
}

/// Snapshot metadata from [`ContainerRuntime::snapshot_list`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// Snapshot name.
    pub name: String,
    /// Identifier of the container that owns the snapshot.
    pub container_id: String,
}

/// Configuration for [`ContainerRuntime::run`].
#[derive(Debug, Clone, Default)]
pub struct RunConfig {
    /// Image repository or name.
    pub image: String,
    /// Image tag; an empty value lets the runtime choose its default.
    pub tag: String,
    /// Command and arguments executed in the container.
    pub command: Vec<String>,
    /// Optional human-readable container name.
    pub name: Option<String>,
    /// Environment entries in `KEY=VALUE` form.
    pub env: Vec<String>,
    /// Bind mounts in runtime-specific syntax.
    pub volumes: Vec<String>,
    /// Optional memory limit in bytes.
    pub memory: Option<u64>,
    /// Optional relative CPU weight.
    pub cpu_weight: Option<u64>,
    /// Optional runtime network mode.
    pub network: Option<String>,
    /// Whether to grant privileged container access.
    pub privileged: bool,
    /// Optional target platform such as `linux/arm64`.
    pub platform: Option<String>,
    /// Whether to remove the container automatically after it exits.
    pub auto_remove: bool,
}

/// Configuration for [`ContainerRuntime::sandbox`].
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Host path of the script to execute.
    pub script: String,
    /// Sandbox image repository or name.
    pub image: String,
    /// Sandbox image tag.
    pub tag: String,
    /// Sandbox memory limit in MiB.
    pub memory_mb: u64,
    /// Sandbox execution timeout in seconds.
    pub timeout_secs: u64,
    /// Bind mounts exposed to the sandbox.
    pub volumes: Vec<String>,
    /// Whether network access is enabled.
    pub network: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            script: String::new(),
            image: "minibox-sandbox".into(),
            tag: "latest".into(),
            memory_mb: DEFAULT_SANDBOX_MEMORY_MB,
            timeout_secs: DEFAULT_SANDBOX_TIMEOUT_SECS,
            volumes: Vec::new(),
            network: false,
        }
    }
}

/// Runtime-agnostic container operations.
///
/// Adapters implement this trait to provide container lifecycle management
/// for any backend (Docker, Podman, minibox).
#[async_trait]
pub trait ContainerRuntime: Send + Sync {
    /// Human-readable runtime name (e.g. "docker", "minibox").
    fn name(&self) -> &str;

    // -- Lifecycle -----------------------------------------------------------

    /// Create and start a container. Returns its ID.
    async fn run(&self, config: &RunConfig) -> Result<ContainerId, ToolError>;

    /// Stop a running container.
    async fn stop(&self, id: &str) -> Result<(), ToolError>;

    /// Pause a running container.
    async fn pause(&self, id: &str) -> Result<(), ToolError>;

    /// Resume a paused container.
    async fn resume(&self, id: &str) -> Result<(), ToolError>;

    /// Remove a stopped container.
    async fn rm(&self, id: &str) -> Result<(), ToolError>;

    /// Remove all stopped containers.
    async fn rm_all(&self) -> Result<(), ToolError>;

    /// List containers.
    async fn ps(&self) -> Result<Vec<ContainerInfo>, ToolError>;

    // -- Execution -----------------------------------------------------------

    /// Execute a command inside a running container.
    async fn exec(&self, id: &str, cmd: &[&str]) -> Result<String, ToolError>;

    /// Fetch log output from a container.
    async fn logs(&self, id: &str) -> Result<String, ToolError>;

    // -- Images --------------------------------------------------------------

    /// Pull an image from the registry.
    async fn pull(&self, image: &str, tag: &str) -> Result<(), ToolError>;

    /// Remove a local image.
    async fn rmi(&self, image_ref: &str) -> Result<(), ToolError>;

    /// Remove unused images.
    async fn prune(&self, dry_run: bool) -> Result<String, ToolError>;

    // -- Sandbox -------------------------------------------------------------

    /// Run a script in a sandboxed container with resource limits.
    ///
    /// Returns `ToolError::ExecutionFailed` with "unsupported" if the
    /// runtime does not support sandboxing.
    async fn sandbox(&self, _config: &SandboxConfig) -> Result<String, ToolError> {
        Err(ToolError::ExecutionFailed(format!(
            "{} does not support sandbox",
            self.name()
        )))
    }

    // -- Snapshots -----------------------------------------------------------

    /// Save a snapshot of a container's state.
    async fn snapshot_save(&self, _id: &str, _name: Option<&str>) -> Result<String, ToolError> {
        Err(ToolError::ExecutionFailed(format!(
            "{} does not support snapshots",
            self.name()
        )))
    }

    /// Restore a container to a saved snapshot.
    async fn snapshot_restore(&self, _id: &str, _name: &str) -> Result<String, ToolError> {
        Err(ToolError::ExecutionFailed(format!(
            "{} does not support snapshots",
            self.name()
        )))
    }

    /// List snapshots for a container.
    async fn snapshot_list(&self, _id: &str) -> Result<Vec<SnapshotInfo>, ToolError> {
        Err(ToolError::ExecutionFailed(format!(
            "{} does not support snapshots",
            self.name()
        )))
    }
}

/// Default command timeout for all container adapters.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default sandbox memory limit in MB.
pub const DEFAULT_SANDBOX_MEMORY_MB: u64 = 512;

/// Default sandbox timeout in seconds.
pub const DEFAULT_SANDBOX_TIMEOUT_SECS: u64 = 60;

/// Number of tab-separated columns in ps output.
const PS_COLUMNS: usize = 4;

/// Run a CLI command with a timeout and return stdout.
///
/// Shared helper used by all CLI-based adapters.
// qual:allow(iosp) reason: "subprocess I/O boundary"
pub(crate) async fn run_cli(
    program: &str,
    args: &[&str],
    env: &[(&str, &str)],
    timeout: Duration,
) -> Result<String, ToolError> {
    use tokio::process::Command;

    let mut cmd = Command::new(program);
    cmd.args(args);
    for (k, v) in env {
        cmd.env(k, v);
    }

    let output = tokio::time::timeout(timeout, cmd.output())
        .await
        .map_err(|_| {
            ToolError::ExecutionFailed(format!("{program} timed out after {}s", timeout.as_secs()))
        })?
        .map_err(|e| ToolError::ExecutionFailed(format!("failed to spawn {program}: {e}")))?;

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
            "{program} exited {}: {}",
            output.status,
            combined.trim()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_config_default_has_empty_image() {
        let cfg = RunConfig::default();
        assert!(cfg.image.is_empty());
        assert!(!cfg.privileged);
        assert!(!cfg.auto_remove);
    }

    #[test]
    fn sandbox_config_default_values() {
        let cfg = SandboxConfig::default();
        assert_eq!(cfg.memory_mb, 512);
        assert_eq!(cfg.timeout_secs, 60);
        assert_eq!(cfg.tag, "latest");
        assert!(!cfg.network);
    }

    #[test]
    fn container_info_serializes_to_json() {
        let info = ContainerInfo {
            id: "abc123".into(),
            name: Some("test".into()),
            image: "alpine:latest".into(),
            status: "running".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("abc123"));
    }
}
