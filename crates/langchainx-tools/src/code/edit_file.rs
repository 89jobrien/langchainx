use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;

use crate::{Tool, ToolError};

pub struct EditFileTool {
    base_dir: PathBuf,
}

impl EditFileTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct EditFileInput {
    path: String,
    old_string: String,
    new_string: String,
}

#[async_trait]
impl Tool for EditFileTool {
    fn name(&self) -> String {
        "EditFile".into()
    }

    fn description(&self) -> String {
        "Replace an exact string in a file. Fails if old_string appears 0 or 2+ times. \
         Input: { \"path\": \"<path>\", \"old_string\": \"<exact text>\", \
         \"new_string\": \"<replacement>\" }. Returns \"ok\"."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Path to the file." },
                "old_string": { "type": "string", "description": "Exact text to replace (must appear exactly once)." },
                "new_string": { "type": "string", "description": "Replacement text." }
            },
            "required": ["path", "old_string", "new_string"],
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
        let parsed: EditFileInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let path = self.base_dir.join(&parsed.path);
        let content = std::fs::read_to_string(&path)
            .map_err(|e| ToolError::ExecutionFailed(format!("cannot read {}: {e}", path.display())))?;

        let count = content.matches(&parsed.old_string).count();
        if count == 0 {
            return Err(ToolError::InvalidInput(
                "old_string not found in file".into(),
            ));
        }
        if count > 1 {
            return Err(ToolError::InvalidInput(format!(
                "old_string appears {count} times; must appear exactly once"
            )));
        }

        let new_content = content.replacen(&parsed.old_string, &parsed.new_string, 1);
        std::fs::write(&path, new_content)
            .map_err(|e| ToolError::ExecutionFailed(format!("cannot write {}: {e}", path.display())))?;

        Ok("ok".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn write_temp_in_dir(dir: &std::path::Path, name: &str, content: &str) {
        std::fs::write(dir.join(name), content).unwrap();
    }

    #[tokio::test]
    async fn replaces_single_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_in_dir(dir.path(), "f.rs", "fn foo() {}\nfn bar() {}");
        let tool = EditFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "f.rs", "old_string": "fn foo()", "new_string": "fn baz()" }))
            .await
            .unwrap();
        assert_eq!(result, "ok");
        let content = std::fs::read_to_string(dir.path().join("f.rs")).unwrap();
        assert!(content.contains("fn baz()"));
        assert!(!content.contains("fn foo()"));
    }

    #[tokio::test]
    async fn errors_on_missing_string() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_in_dir(dir.path(), "f.rs", "fn foo() {}");
        let tool = EditFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "f.rs", "old_string": "fn missing()", "new_string": "x" }))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not found"), "got: {err}");
    }

    #[tokio::test]
    async fn errors_on_duplicate_string() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_in_dir(dir.path(), "f.rs", "foo foo bar");
        let tool = EditFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "f.rs", "old_string": "foo", "new_string": "x" }))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("2 times"), "got: {err}");
    }

    #[tokio::test]
    async fn errors_on_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let tool = EditFileTool::new(dir.path());
        let result = tool
            .run(json!({ "path": "no_file.rs", "old_string": "x", "new_string": "y" }))
            .await;
        assert!(result.is_err());
    }

}
