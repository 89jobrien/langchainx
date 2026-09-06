//! Full-file writes with automatic parent-directory creation.
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

use super::validate_new_path;
use crate::{Tool, ToolError};

/// Writes complete file contents, resolving relative paths from a base directory.
///
/// Paths are confined to the configured base directory.
pub struct WriteFileTool {
    base_dir: PathBuf,
}

impl WriteFileTool {
    /// Creates a file writer that resolves relative paths from `base_dir`.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct WriteFileInput {
    path: String,
    content: String,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> String {
        "WriteFile".into()
    }

    fn description(&self) -> String {
        "Write content to a file, creating parent directories as needed. \
         Input: { \"path\": \"<path>\", \"content\": \"<full content>\" }. Returns \"ok\"."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to write to." },
                "content": { "type": "string", "description": "Full file content." }
            },
            "required": ["path", "content"],
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
        let parsed: WriteFileInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let path = validate_new_path(&self.base_dir, &parsed.path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::ExecutionFailed(format!("cannot create dirs: {e}")))?;
        }
        std::fs::write(&path, &parsed.content).map_err(|e| {
            ToolError::ExecutionFailed(format!("cannot write {}: {e}", path.display()))
        })?;

        Ok("ok".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn writes_file_and_returns_ok() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "hello.txt", "content": "hello world" }))
            .await
            .unwrap();
        assert_eq!(result, "ok");
        let content = std::fs::read_to_string(dir.path().join("hello.txt")).unwrap();
        assert_eq!(content, "hello world");
    }

    #[tokio::test]
    async fn creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "a/b/c.txt", "content": "nested" }))
            .await
            .unwrap();
        assert_eq!(result, "ok");
        let content = std::fs::read_to_string(dir.path().join("a/b/c.txt")).unwrap();
        assert_eq!(content, "nested");
    }

    #[tokio::test]
    async fn invalid_input_errors() {
        let dir = tempfile::tempdir().unwrap();
        let tool = WriteFileTool::new(dir.path());
        let result = tool.run(json!({ "path": "x" })).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_parent_directory_traversal() {
        let parent = tempfile::tempdir().unwrap();
        let base = parent.path().join("base");
        std::fs::create_dir(&base).unwrap();
        let tool = WriteFileTool::new(&base);

        let result = tool
            .run(json!({ "path": "../escaped.txt", "content": "escaped" }))
            .await;

        assert!(matches!(result, Err(ToolError::InvalidInput(_))));
        assert!(!parent.path().join("escaped.txt").exists());
    }
}
