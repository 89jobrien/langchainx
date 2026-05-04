use std::path::PathBuf;
use std::sync::Arc;

use crate::Tool;

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
pub fn coding_tools(base_dir: impl Into<PathBuf>) -> Vec<Arc<dyn Tool>> {
    let base: PathBuf = base_dir.into();

    #[allow(unused_mut)]
    let mut tools: Vec<Arc<dyn Tool>> = vec![
        Arc::new(ReadFileTool::new(base.clone())),
        Arc::new(WriteFileTool::new(base.clone())),
        Arc::new(EditFileTool::new(base.clone())),
        Arc::new(GlobTool::new(base.clone())),
        Arc::new(GrepTool::new(base.clone())),
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
        let mut expected = 5;
        #[cfg(feature = "bash-tool")]
        { expected += 1; }
        #[cfg(feature = "nu-tool")]
        { expected += 1; }
        assert_eq!(tools.len(), expected);
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
