//! File, search, shell, and coding-agent tool adapters.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::Tool;

/// Canonicalizes `path` and rejects values that escape `base_dir`.
pub fn validate_path(base_dir: &Path, path: &str) -> Result<PathBuf, crate::ToolError> {
    let joined = base_dir.join(path);
    let canonical = joined
        .canonicalize()
        .map_err(|e| crate::ToolError::InvalidInput(format!("path not found: {e}")))?;
    let base_canonical = base_dir
        .canonicalize()
        .map_err(|e| crate::ToolError::InvalidInput(format!("base dir not found: {e}")))?;
    if !canonical.starts_with(&base_canonical) {
        return Err(crate::ToolError::InvalidInput(
            "path escapes base directory".to_string(),
        ));
    }
    Ok(canonical)
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
