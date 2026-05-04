use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

use crate::{Tool, ToolError};

const MAX_RESULTS: usize = 200;

pub struct GrepTool {
    base_dir: PathBuf,
}

impl GrepTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GrepInput {
    pattern: String,
    path: String,
    #[serde(default)]
    glob: Option<String>,
}

fn glob_matches(file_name: &str, pattern: &str) -> bool {
    // Simple glob: support * and ? wildcards
    // Convert glob to regex for matching
    let mut regex_str = String::from("^");
    for ch in pattern.chars() {
        match ch {
            '*' => regex_str.push_str(".*"),
            '?' => regex_str.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                regex_str.push('\\');
                regex_str.push(ch);
            }
            c => regex_str.push(c),
        }
    }
    regex_str.push('$');
    Regex::new(&regex_str)
        .map(|r| r.is_match(file_name))
        .unwrap_or(false)
}

fn search_file(
    path: &Path,
    re: &Regex,
    results: &mut Vec<String>,
    glob_filter: Option<&str>,
) -> std::io::Result<()> {
    if results.len() >= MAX_RESULTS {
        return Ok(());
    }
    if path.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            search_file(&entry.path(), re, results, glob_filter)?;
            if results.len() >= MAX_RESULTS {
                break;
            }
        }
    } else if path.is_file() {
        // Apply glob filter on file name if specified
        if let Some(filter) = glob_filter {
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if !glob_matches(file_name, filter) {
                return Ok(());
            }
        }
        // Skip binary files: try reading as UTF-8
        if let Ok(content) = std::fs::read_to_string(path) {
            for (lineno, line) in content.lines().enumerate() {
                if re.is_match(line) {
                    results.push(format!("{}:{}: {}", path.display(), lineno + 1, line));
                    if results.len() >= MAX_RESULTS {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> String {
        "Grep".into()
    }

    fn description(&self) -> String {
        "Search for a regex pattern in files. \
         Input: { \"pattern\": \"<regex>\", \"path\": \"<dir or file>\", \
         \"glob\": \"<optional file filter>\" }. \
         Returns file:line: content lines (up to 200)."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern to search for." },
                "path": { "type": "string", "description": "Directory or file to search." },
                "glob": { "type": "string", "description": "Optional filename glob filter (e.g. '*.rs')." }
            },
            "required": ["pattern", "path"],
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
        let parsed: GrepInput = serde_json::from_value(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let re = Regex::new(&parsed.pattern)
            .map_err(|e| ToolError::InvalidInput(format!("invalid regex: {e}")))?;

        let search_path = self.base_dir.join(&parsed.path);
        let mut results = Vec::new();
        search_file(&search_path, &re, &mut results, parsed.glob.as_deref())
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(results.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn finds_matching_lines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "hello world\nfoo bar\nhello again").unwrap();
        let tool = GrepTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "hello", "path": "a.txt" }))
            .await
            .unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("hello world"));
    }

    #[tokio::test]
    async fn no_matches_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "nothing here").unwrap();
        let tool = GrepTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "xyz123", "path": "a.txt" }))
            .await
            .unwrap();
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn glob_filter_applied() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.rs"), "match me").unwrap();
        std::fs::write(dir.path().join("b.txt"), "match me").unwrap();
        let tool = GrepTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "match me", "path": ".", "glob": "*.rs" }))
            .await
            .unwrap();
        assert!(result.contains("a.rs"));
        assert!(!result.contains("b.txt"));
    }

    #[tokio::test]
    async fn invalid_regex_errors() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GrepTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "[invalid(", "path": "." }))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn includes_line_numbers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f.txt"), "a\nb\nc\nb").unwrap();
        let tool = GrepTool::new(dir.path());
        let result = tool
            .run(json!({ "pattern": "^b$", "path": "f.txt" }))
            .await
            .unwrap();
        assert!(result.contains(":2:"));
        assert!(result.contains(":4:"));
    }
}
