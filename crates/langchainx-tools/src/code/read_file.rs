use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::{Tool, ToolError};

pub struct ReadFileTool {
    base_dir: PathBuf,
}

impl ReadFileTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReadFileInput {
    path: String,
    #[serde(default)]
    offset: Option<u64>,
    #[serde(default)]
    limit: Option<u64>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> String {
        "ReadFile".into()
    }

    fn description(&self) -> String {
        "Read a file from disk. Input: { \"path\": \"<path>\", \"offset\": <optional line>, \
         \"limit\": <optional lines> }. Returns file content."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file." },
                "offset": { "type": "integer", "description": "Starting line number (0-indexed)." },
                "limit": { "type": "integer", "description": "Maximum number of lines to return." }
            },
            "required": ["path"],
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
        let parsed: ReadFileInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let path = self.base_dir.join(&parsed.path);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ToolError::ExecutionFailed(format!("cannot read {}: {e}", path.display())))?;

        let lines: Vec<&str> = content.lines().collect();
        let offset = parsed.offset.unwrap_or(0) as usize;
        let slice = if let Some(limit) = parsed.limit {
            &lines[offset.min(lines.len())..(offset + limit as usize).min(lines.len())]
        } else {
            &lines[offset.min(lines.len())..]
        };

        Ok(slice.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    #[tokio::test]
    async fn reads_full_file() {
        let f = write_temp("line1\nline2\nline3");
        let tool = ReadFileTool::new("/");
        let abs = f.path().to_string_lossy();
        // Use absolute path by setting base_dir to /
        let result = tool
            .run(json!({ "path": abs.trim_start_matches('/') }))
            .await
            .unwrap();
        assert!(result.contains("line1"));
        assert!(result.contains("line3"));
    }

    #[tokio::test]
    async fn reads_with_offset_and_limit() {
        let f = write_temp("a\nb\nc\nd\ne");
        let tool = ReadFileTool::new("/");
        let abs = f.path().to_string_lossy();
        let result = tool
            .run(json!({ "path": abs.trim_start_matches('/'), "offset": 1, "limit": 2 }))
            .await
            .unwrap();
        assert_eq!(result, "b\nc");
    }

    #[tokio::test]
    async fn missing_file_errors() {
        let tool = ReadFileTool::new("/tmp");
        let result = tool
            .run(json!({ "path": "definitely_does_not_exist_xyz.txt" }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn invalid_input_errors() {
        let tool = ReadFileTool::new("/tmp");
        let result = tool.run(json!({ "not_path": "x" })).await;
        assert!(result.is_err());
    }
}
