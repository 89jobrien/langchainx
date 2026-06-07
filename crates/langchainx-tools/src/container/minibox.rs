//! Minibox (`mbx`) adapter for [`ContainerRuntime`].

use async_trait::async_trait;
use std::path::PathBuf;
use std::time::Duration;

use super::{
    ContainerId, ContainerInfo, ContainerRuntime, RunConfig, SandboxConfig, SnapshotInfo,
    DEFAULT_TIMEOUT, run_cli,
};
use crate::ToolError;

/// Container runtime adapter that shells out to `mbx`.
///
/// Supports the full minibox feature set including sandbox and snapshots.
pub struct MiniboxRuntime {
    mbx_path: PathBuf,
    socket_path: Option<PathBuf>,
    timeout: Duration,
}

impl MiniboxRuntime {
    pub fn new() -> Self {
        Self {
            mbx_path: PathBuf::from("mbx"),
            socket_path: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Auto-detect `mbx` on PATH. Returns `None` if not found.
    pub fn detect() -> Option<Self> {
        if which::which("mbx").is_ok() {
            Some(Self::new())
        } else {
            None
        }
    }

    pub fn with_mbx_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.mbx_path = path.into();
        self
    }

    pub fn with_socket_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.socket_path = Some(path.into());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn bin(&self) -> &str {
        self.mbx_path.to_str().unwrap_or("mbx")
    }

    // qual:allow(iosp) reason: "CLI adapter I/O boundary"
    async fn cmd(&self, args: &[&str]) -> Result<String, ToolError> {
        let env: Vec<(&str, &str)> = self
            .socket_path
            .as_ref()
            .map(|s| vec![("MINIBOX_SOCKET", s.to_str().unwrap_or(""))])
            .unwrap_or_default();
        run_cli(self.bin(), args, &env, self.timeout).await
    }
}

impl Default for MiniboxRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ContainerRuntime for MiniboxRuntime {
    fn name(&self) -> &str {
        "minibox"
    }

    // qual:allow(iosp) reason: "CLI adapter builds args then delegates"
    async fn run(&self, config: &RunConfig) -> Result<ContainerId, ToolError> {
        let tag = if config.tag.is_empty() {
            "latest"
        } else {
            &config.tag
        };
        let network = config.network.as_deref().unwrap_or("none");

        let mut args = vec![
            "run".to_string(),
            "--tag".to_string(),
            tag.to_string(),
            "--network".to_string(),
            network.to_string(),
        ];

        if let Some(m) = config.memory {
            args.push("--memory".into());
            args.push(m.to_string());
        }
        if let Some(w) = config.cpu_weight {
            args.push("--cpu-weight".into());
            args.push(w.to_string());
        }
        if config.privileged {
            args.push("--privileged".into());
        }
        if config.auto_remove {
            args.push("--rm".into());
        }
        for v in &config.volumes {
            args.push("-v".into());
            args.push(v.clone());
        }
        for e in &config.env {
            args.push("-e".into());
            args.push(e.clone());
        }
        if let Some(ref n) = config.name {
            args.push("--name".into());
            args.push(n.clone());
        }
        if let Some(ref p) = config.platform {
            args.push("--platform".into());
            args.push(p.clone());
        }
        args.push(config.image.clone());
        if !config.command.is_empty() {
            args.push("--".into());
            args.extend(config.command.clone());
        }

        let refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let out = self.cmd(&refs).await?;
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
        self.cmd(&["resume", id]).await?;
        Ok(())
    }

    async fn rm(&self, id: &str) -> Result<(), ToolError> {
        self.cmd(&["rm", id]).await?;
        Ok(())
    }

    async fn rm_all(&self) -> Result<(), ToolError> {
        self.cmd(&["rm", "--all"]).await?;
        Ok(())
    }

    async fn ps(&self) -> Result<Vec<ContainerInfo>, ToolError> {
        let out = self.cmd(&["ps"]).await?;
        let containers = out
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| {
                let parts: Vec<&str> = line.splitn(4, '\t').collect();
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
        let mut args = vec!["exec", id, "--"];
        args.extend(cmd);
        self.cmd(&args).await
    }

    async fn logs(&self, id: &str) -> Result<String, ToolError> {
        self.cmd(&["logs", id]).await
    }

    async fn pull(&self, image: &str, tag: &str) -> Result<(), ToolError> {
        self.cmd(&["pull", image, "--tag", tag]).await?;
        Ok(())
    }

    async fn rmi(&self, image_ref: &str) -> Result<(), ToolError> {
        self.cmd(&["rmi", image_ref]).await?;
        Ok(())
    }

    async fn prune(&self, dry_run: bool) -> Result<String, ToolError> {
        if dry_run {
            self.cmd(&["prune", "--dry-run"]).await
        } else {
            self.cmd(&["prune"]).await
        }
    }

    async fn sandbox(&self, config: &SandboxConfig) -> Result<String, ToolError> {
        let mem = config.memory_mb.to_string();
        let timeout = config.timeout_secs.to_string();
        let mut args = vec![
            "sandbox",
            &config.script,
            "--image",
            &config.image,
            "--tag",
            &config.tag,
            "--memory-mb",
            &mem,
            "--timeout",
            &timeout,
        ];
        let vol_flags: Vec<String> = config
            .volumes
            .iter()
            .flat_map(|v| vec!["-v".to_string(), v.clone()])
            .collect();
        let vol_refs: Vec<&str> = vol_flags.iter().map(String::as_str).collect();
        args.extend(&vol_refs);
        if config.network {
            args.push("--network");
        }
        self.cmd(&args).await
    }

    async fn snapshot_save(
        &self,
        id: &str,
        name: Option<&str>,
    ) -> Result<String, ToolError> {
        let mut args = vec!["snapshot", "save", id];
        if let Some(n) = name {
            args.push(n);
        }
        self.cmd(&args).await
    }

    async fn snapshot_restore(
        &self,
        id: &str,
        name: &str,
    ) -> Result<String, ToolError> {
        self.cmd(&["snapshot", "restore", id, name]).await
    }

    async fn snapshot_list(&self, id: &str) -> Result<Vec<SnapshotInfo>, ToolError> {
        let out = self.cmd(&["snapshot", "list", id]).await?;
        let snapshots = out
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|line| SnapshotInfo {
                name: line.trim().to_string(),
                container_id: id.to_string(),
            })
            .collect();
        Ok(snapshots)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minibox_runtime_default_name() {
        let rt = MiniboxRuntime::new();
        assert_eq!(rt.name(), "minibox");
    }

    #[test]
    fn minibox_runtime_custom_path() {
        let rt = MiniboxRuntime::new().with_mbx_path("/usr/local/bin/mbx");
        assert_eq!(rt.mbx_path, PathBuf::from("/usr/local/bin/mbx"));
    }

    #[test]
    fn minibox_runtime_socket_path() {
        let rt = MiniboxRuntime::new()
            .with_socket_path("/run/minibox/miniboxd.sock");
        assert_eq!(
            rt.socket_path,
            Some(PathBuf::from("/run/minibox/miniboxd.sock"))
        );
    }

    #[test]
    fn minibox_runtime_timeout() {
        let rt = MiniboxRuntime::new().with_timeout(Duration::from_secs(120));
        assert_eq!(rt.timeout, Duration::from_secs(120));
    }
}
