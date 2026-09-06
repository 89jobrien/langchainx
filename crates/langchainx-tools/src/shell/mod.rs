//! File, search, shell, and coding-agent tool adapters.
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

#[cfg(any(feature = "bash-tool", feature = "nu-tool"))]
use std::{process::Stdio, time::Duration};
#[cfg(any(feature = "bash-tool", feature = "nu-tool"))]
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

use crate::Tool;

/// Canonicalizes `path` and rejects values that escape `base_dir`.
pub fn validate_path(base_dir: &Path, path: &str) -> Result<PathBuf, crate::ToolError> {
    let base_canonical = canonical_base(base_dir)?;
    let canonical = base_dir
        .join(path)
        .canonicalize()
        .map_err(|e| crate::ToolError::InvalidInput(format!("path not found: {e}")))?;
    if !canonical.starts_with(&base_canonical) {
        return Err(crate::ToolError::InvalidInput(
            "path escapes base directory".to_string(),
        ));
    }
    Ok(canonical)
}

/// Resolves a potentially new path while preventing traversal outside `base_dir`.
pub fn validate_new_path(base_dir: &Path, path: &str) -> Result<PathBuf, crate::ToolError> {
    let relative = Path::new(path);
    validate_relative_components(relative)?;

    let base_canonical = canonical_base(base_dir)?;
    let target = base_canonical.join(relative);
    let mut ancestor = target.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or_else(|| {
            crate::ToolError::InvalidInput("path has no existing ancestor".to_string())
        })?;
    }

    let canonical_ancestor = ancestor
        .canonicalize()
        .map_err(|e| crate::ToolError::InvalidInput(format!("invalid path: {e}")))?;
    if !canonical_ancestor.starts_with(&base_canonical) {
        return Err(crate::ToolError::InvalidInput(
            "path escapes base directory".to_string(),
        ));
    }

    Ok(target)
}

/// Builds a confined glob pattern rooted at `base_dir`.
pub fn validate_glob_pattern(base_dir: &Path, pattern: &str) -> Result<PathBuf, crate::ToolError> {
    let relative = Path::new(pattern);
    validate_relative_components(relative)?;
    Ok(canonical_base(base_dir)?.join(relative))
}

fn canonical_base(base_dir: &Path) -> Result<PathBuf, crate::ToolError> {
    base_dir
        .canonicalize()
        .map_err(|e| crate::ToolError::InvalidInput(format!("base dir not found: {e}")))
}

fn validate_relative_components(path: &Path) -> Result<(), crate::ToolError> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(crate::ToolError::InvalidInput(
            "path must stay within the base directory".to_string(),
        ));
    }
    Ok(())
}

#[cfg(any(feature = "bash-tool", feature = "nu-tool"))]
pub(crate) struct BoundedOutput {
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) status: std::process::ExitStatus,
}

#[cfg(any(feature = "bash-tool", feature = "nu-tool"))]
pub(crate) async fn run_bounded_command(
    mut command: Command,
    timeout: Duration,
    max_output_bytes: usize,
) -> Result<BoundedOutput, crate::ToolError> {
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .map_err(|error| crate::ToolError::ExecutionFailed(error.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        crate::ToolError::ExecutionFailed("failed to capture command stdout".to_string())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        crate::ToolError::ExecutionFailed("failed to capture command stderr".to_string())
    })?;
    let per_stream_limit = max_output_bytes / 2;
    let stdout_task = tokio::spawn(read_capped(stdout, per_stream_limit));
    let stderr_task = tokio::spawn(read_capped(stderr, per_stream_limit));

    let status = match tokio::time::timeout(timeout, child.wait()).await {
        Ok(status) => {
            status.map_err(|error| crate::ToolError::ExecutionFailed(error.to_string()))?
        }
        Err(_) => {
            let _ = child.kill().await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(crate::ToolError::ExecutionFailed(
                "command timed out".to_string(),
            ));
        }
    };
    let stdout = stdout_task
        .await
        .map_err(|error| crate::ToolError::ExecutionFailed(error.to_string()))??;
    let stderr = stderr_task
        .await
        .map_err(|error| crate::ToolError::ExecutionFailed(error.to_string()))??;

    Ok(BoundedOutput {
        stdout,
        stderr,
        status,
    })
}

#[cfg(any(feature = "bash-tool", feature = "nu-tool"))]
async fn read_capped<R: AsyncRead + Unpin>(
    mut reader: R,
    limit: usize,
) -> Result<Vec<u8>, crate::ToolError> {
    let mut captured = Vec::with_capacity(limit.min(8192));
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| crate::ToolError::ExecutionFailed(error.to_string()))?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    Ok(captured)
}

#[cfg(feature = "bash-tool")]
pub mod bash;
#[cfg(feature = "bash-tool")]
pub use bash::BashTool;

pub mod read_file;
pub use read_file::ReadFileTool;

pub mod write_file;
pub use write_file::WriteFileTool;

pub mod edit_file;
pub use edit_file::EditFileTool;

pub mod glob;
pub use glob::GlobTool;

pub mod grep;
pub use grep::GrepTool;

#[cfg(feature = "nu-tool")]
pub mod nu;
#[cfg(feature = "nu-tool")]
pub use nu::NuTool;

/// Returns the standard set of coding agent tools rooted at `base_dir`.
///
/// With `bash-tool` feature enabled, `BashTool` is also included.
/// With `nu-tool` feature enabled, `NuTool` is also included.
///
/// Also includes Rust-specific CLI tools from sibling modules:
/// - `RustqualTool` (requires `rustqual` binary)
/// - `KaniTool` (requires `kani` binary)
/// - `ClippyTool` (requires `cargo` with clippy component)
/// - `CargoTestTool` (requires `cargo` or `cargo-nextest`)
/// - `AgentlintTool` (requires `agentlint` binary)
///
/// If a CLI binary is missing, the tool returns `ToolError::ExecutionFailed`.
pub fn coding_tools(base_dir: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let base: PathBuf = base_dir.into();

    #[allow(unused_mut)]
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadFileTool::new(base.clone())),
        Arc::new(WriteFileTool::new(base.clone())),
        Arc::new(EditFileTool::new(base.clone())),
        Arc::new(GlobTool::new(base.clone())),
        Arc::new(GrepTool::new(base.clone())),
        Arc::new(crate::rustqual::RustqualTool::new(base.clone())),
        Arc::new(crate::kani::KaniTool::new(base.clone())),
        Arc::new(crate::cargo::ClippyTool::new(base.clone())),
        Arc::new(crate::cargo::CargoTestTool::new(base.clone())),
        Arc::new(crate::agentlint::AgentlintTool::new(base.clone())),
    ];

    #[cfg(feature = "bash-tool")]
    tools.push(Arc::new(BashTool));

    #[cfg(feature = "nu-tool")]
    tools.push(Arc::new(NuTool::new()));

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coding_tools_returns_expected_count() {
        let tools = coding_tools(".");
        let expected =
            10 + usize::from(cfg!(feature = "bash-tool")) + usize::from(cfg!(feature = "nu-tool"));
        assert_eq!(tools.len(), expected);
    }

    #[test]
    fn validate_path_rejects_traversal() {
        let base = std::env::temp_dir();
        let result = validate_path(&base, "../../etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn validate_path_accepts_valid() {
        let base = std::env::temp_dir();
        let result = validate_path(&base, ".");
        assert!(result.is_ok());
    }

    #[test]
    fn tool_names_are_unique() {
        let tools = coding_tools(".");
        let mut names: Vec<String> = tools.iter().map(|t| t.name()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), tools.len());
    }
}
