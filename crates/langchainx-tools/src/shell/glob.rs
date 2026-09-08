//! Bounded file discovery with glob patterns.
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

use super::{validate_glob_pattern, validate_path};
use crate::{Tool, ToolError};

const MAX_RESULTS: usize = 200;

/// Finds files matching a glob pattern, resolving relative patterns from a base directory.
///
/// Patterns and optional paths are confined to the configured base directory.
pub struct GlobTool {
    base_dir: PathBuf,
}

impl GlobTool {
    /// Creates a glob tool that resolves relative patterns from `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GlobInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

impl Tool for GlobTool {
    fn name(&self) -> String {
        "Glob".into()
    }

    fn description(&self) -> String {
        "Find files matching a glob pattern. \
         Input: { \"pattern\": \"<glob>\", \"path\": \"<optional base dir>\" }. \
         Returns newline-separated paths (up to 200)."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern." },
                "path": { "type": "string", "description": "Optional base directory." }
            },
            "required": ["pattern"],
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
        let parsed: GlobInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let base = if let Some(path) = &parsed.path {
            validate_path(&self.base_dir, path)?
        } else {
            validate_path(&self.base_dir, ".")?
        };

        let full_pattern = validate_glob_pattern(&base, &parsed.pattern)?;
        let pattern_str = full_pattern.to_string_lossy();

        let mut paths = Vec::new();
        for entry in glob::glob(&pattern_str)
            .map_err(|e| ToolError::InvalidInput(format!("invalid glob: {e}")))?
        {
            let path = entry.map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
            let confined = validate_path(&self.base_dir, &path.to_string_lossy())?;
            paths.push(confined.to_string_lossy().into_owned());
            if paths.len() >= MAX_RESULTS {
                break;
            }
        }

        Ok(paths.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn finds_files_by_extension() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "").unwrap();
        std::fs::write(dir.path().join("b.rs"), "").unwrap();
        std::fs::write(dir.path().join("c.txt"), "").unwrap();

        let tool = GlobTool::new(dir.path());
        let result = tool.run(json!({ "pattern": "*.rs" })).await.unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines.iter().all(|l| l.ends_with(".rs")));
    }

    #[tokio::test]
    async fn returns_empty_for_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GlobTool::new(dir.path());
        let result = tool.run(json!({ "pattern": "*.xyz" })).await.unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn invalid_input_errors() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GlobTool::new(dir.path());
        let result = tool.run(json!({ "not_pattern": "x" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn uses_optional_path_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/x.rs"), "").unwrap();
        std::fs::write(dir.path().join("y.rs"), "").unwrap();

        let tool = GlobTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "*.rs", "path": "sub" }))
            .await
            .unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("sub"));
    }

    #[tokio::test]
    async fn rejects_parent_directory_traversal() {
        let parent = tempfile::tempdir().unwrap();
        let base = parent.path().join("base");
        std::fs::create_dir(&base).unwrap();
        std::fs::write(parent.path().join("outside.rs"), "").unwrap();
        let tool = GlobTool::new(&base);

        let result = tool.run(json!({ "pattern": "../*.rs" })).await;

        assert!(matches!(result, Err(ToolError::InvalidInput(_))));
    }
}
