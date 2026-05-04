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

// NuTool — placeholder, tracked in issue #78
// #[cfg(feature = "nu-tool")]
// pub mod nu;

/// Returns the standard set of coding agent tools rooted at `base_dir`.
///
/// With `bash-tool` feature enabled, `BashTool` is also included.
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

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coding_tools_returns_expected_count() {
        let tools = coding_tools(".");
        // 5 tools always; BashTool added with bash-tool feature
        #[cfg(feature = "bash-tool")]
        assert_eq!(tools.len(), 6);
        #[cfg(not(feature = "bash-tool"))]
        assert_eq!(tools.len(), 5);
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
