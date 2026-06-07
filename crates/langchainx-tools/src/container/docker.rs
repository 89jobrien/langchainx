//! Docker/Podman adapter for [`ContainerRuntime`].

use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

use super::{
    ContainerId, ContainerInfo, ContainerRuntime, RunConfig, DEFAULT_TIMEOUT,
    run_cli,
};
use crate::ToolError;

/// Container runtime adapter that shells out to `docker` or `podman`.
///
/// Both CLIs share the same command interface, so a single adapter covers
/// both. Use [`DockerRuntime::detect`] to find whichever is available.
pub struct DockerRuntime {
    binary: PathBuf,
    timeout: Duration,
}

impl DockerRuntime {
    /// Create with an explicit binary path.
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Auto-detect `docker` or `podman` on PATH.
    /// Returns `None` if neither is found.
    // qual:allow(iosp) reason: "runtime detection inherently mixes probing with selection"
    pub fn detect() -> Option<Self> {
        for name in ["docker", "podman"] {
            if which::which(name).is_ok() {
                return Some(Self::new(name));
            }
        }
        None
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn bin(&self) -> &str {
        self.binary.to_str().unwrap_or("docker")
    }

    async fn cmd(&self, args: &[&str]) -> Result<String, ToolError> {
        run_cli(self.bin(), args, &[], self.timeout).await
    }
}

#[async_trait]
impl ContainerRuntime for DockerRuntime {
    fn name(&self) -> &str {
        self.bin()
    }

    async fn run(&self, config: &RunConfig) -> Result<ContainerId, ToolError> {
        let mut args = vec!["run", "-d"];

        let tag_ref;
        let image_ref = if config.tag.is_empty() {
            &config.image
        } else {
            tag_ref = format!("{}:{}", config.image, config.tag);
            &tag_ref
        };

        let name_flag;
        if let Some(ref n) = config.name {
            name_flag = n.clone();
            args.push("--name");
            args.push(&name_flag);
        }

        let net_flag;
        if let Some(ref net) = config.network {
            net_flag = format!("--network={net}");
            args.push(&net_flag);
        }

        let mem_flag;
        if let Some(m) = config.memory {
            mem_flag = format!("--memory={m}");
            args.push(&mem_flag);
        }

        if config.privileged {
            args.push("--privileged");
        }
        if config.auto_remove {
            args.push("--rm");
        }

        let vol_flags: Vec<String> =
            config.volumes.iter().map(|v| format!("-v={v}")).collect();
        for v in &vol_flags {
            args.push(v);
        }

        let env_flags: Vec<String> =
            config.env.iter().map(|e| format!("-e={e}")).collect();
        for e in &env_flags {
            args.push(e);
        }

        let platform_flag;
        if let Some(ref p) = config.platform {
            platform_flag = format!("--platform={p}");
            args.push(&platform_flag);
        }

        args.push(image_ref);

        let cmd_refs: Vec<&str> = config.command.iter().map(String::as_str).collect();
        args.extend(&cmd_refs);

        let out = self.cmd(&args).await?;
        Ok(out.trim().to_string())
    }

    async fn stop(&self, id: &str) -> Result<(), ToolError> {
        self.cmd(&["stop", id]).await?;
        Ok(())
    }

    async fn pause(&self, id: &str) -> Result<(), ToolError> {
        self.cmd(&["pause", id]).await?;
        Ok(())
    }

    async fn resume(&self, id: &str) -> Result<(), ToolError> {
        self.cmd(&["unpause", id]).await?;
        Ok(())
    }

    async fn rm(&self, id: &str) -> Result<(), ToolError> {
        self.cmd(&["rm", id]).await?;
        Ok(())
    }

    async fn rm_all(&self) -> Result<(), ToolError> {
        self.cmd(&["container", "prune", "-f"]).await?;
        Ok(())
    }

    async fn ps(&self) -> Result<Vec<ContainerInfo>, ToolError> {
        let out = self
            .cmd(&[
                "ps",
                "-a",
                "--format",
                "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}",
            ])
            .await?;

        let containers = out
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(super::PS_COLUMNS, '\t').collect();
                ContainerInfo {
                    id: parts.first().unwrap_or(&"").to_string(),
                    name: parts.get(1).map(|s| s.to_string()),
                    image: parts.get(2).unwrap_or(&"").to_string(),
                    status: parts.get(3).unwrap_or(&"").to_string(),
                }
            })
            .collect();

        Ok(containers)
    }

    async fn exec(&self, id: &str, cmd: &[&str]) -> Result<String, ToolError> {
        let mut args = vec!["exec", id];
        args.extend(cmd);
        self.cmd(&args).await
    }

    async fn logs(&self, id: &str) -> Result<String, ToolError> {
        self.cmd(&["logs", id]).await
    }

    async fn pull(&self, image: &str, tag: &str) -> Result<(), ToolError> {
        let full = format!("{image}:{tag}");
        self.cmd(&["pull", &full]).await?;
        Ok(())
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), ToolError> {
        self.cmd(&["rmi", image_ref]).await?;
        Ok(())
    }

    async fn prune(&self, dry_run: bool) -> Result<String, ToolError> {
        if dry_run {
            self.cmd(&["image", "ls", "--filter", "dangling=true"])
                .await
        } else {
            self.cmd(&["image", "prune", "-f"]).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_runtime_name() {
        let rt = DockerRuntime::new("docker");
        assert_eq!(rt.name(), "docker");
    }

    #[test]
    fn podman_runtime_name() {
        let rt = DockerRuntime::new("podman");
        assert_eq!(rt.name(), "podman");
    }

    #[test]
    fn docker_runtime_with_timeout() {
        let rt = DockerRuntime::new("docker").with_timeout(Duration::from_secs(60));
        assert_eq!(rt.timeout, Duration::from_secs(60));
    }
}
