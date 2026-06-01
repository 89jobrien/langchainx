use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::process::Command;

use crate::{Tool, ToolError};

/// Tool adapter for `cargo test` / `cargo nextest run`.
///
/// Runs tests and returns structured output. Prefers nextest if available.
pub struct CargoTestTool {
    base_dir: PathBuf,
    use_nextest: bool,
}

impl CargoTestTool {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            use_nextest: true,
        }
    }

    pub fn with_nextest(mut self, enabled: bool) -> Self {
        self.use_nextest = enabled;
        self
    }
}

#[derive(Debug, Deserialize)]
struct CargoTestInput {
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    test_name: Option<String>,
    #[serde(default)]
    all_features: bool,
    #[serde(default)]
    no_capture: bool,
}

#[async_trait]
impl Tool for CargoTestTool {
    fn name(&self) -> String {
        "CargoTest".into()
    }

    fn description(&self) -> String {
        "Run Rust tests via cargo test or cargo nextest. \
         Can target specific packages or test names. \
         Returns test output with pass/fail results."
            .into()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "package": {
                    "type": "string",
                    "description": "Specific package to test."
                },
                "test_name": {
                    "type": "string",
                    "description": "Filter to tests matching this name."
                },
                "all_features": {
                    "type": "boolean",
                    "description": "Enable all feature flags.",
                    "default": false
                },
                "no_capture": {
                    "type": "boolean",
                    "description": "Show stdout/stderr from tests.",
                    "default": false
                }
            },
            "required": [],
            "additionalProperties": false
        })
    }

    async fn parse_input(&self, input: &str) -> Value {
        serde_json::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_string()))
    }

    // qual:allow(iosp) reason: "tool I/O boundary"
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let parsed: CargoTestInput =
            serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let (cargo_args, test_args) = build_args(&parsed, self.use_nextest);

        let mut cmd = Command::new("cargo");
        for arg in &cargo_args {
            cmd.arg(arg);
        }
        cmd.current_dir(&self.base_dir);

        if !test_args.is_empty() {
            cmd.arg("--");
            for arg in &test_args {
                cmd.arg(arg);
            }
        }

        let output = cmd
            .output()
            .map_err(|e| ToolError::ExecutionFailed(format!("failed to run cargo test: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let mut result = stdout;
        if !output.status.success() {
            result.push_str("\n--- stderr ---\n");
            result.push_str(&stderr);
        }

        Ok(result)
    }
}

/// Build the argument list for `cargo test` / `cargo nextest run`.
///
/// Returns `(cargo_args, separator_args)` where `separator_args` are placed
/// after `--` for non-nextest invocations.
fn build_args(parsed: &CargoTestInput, use_nextest: bool) -> (Vec<String>, Vec<String>) {
    let mut cargo_args = Vec::new();
    let mut test_args = Vec::new();

    if use_nextest {
        cargo_args.push("nextest".into());
        cargo_args.push("run".into());
    } else {
        cargo_args.push("test".into());
    }

    if let Some(ref pkg) = parsed.package {
        cargo_args.push("-p".into());
        cargo_args.push(pkg.clone());
    }
    if parsed.all_features {
        cargo_args.push("--all-features".into());
    }

    if parsed.no_capture {
        if use_nextest {
            cargo_args.push("--no-capture".into());
        } else {
            test_args.push("--nocapture".into());
        }
    }

    if let Some(ref name) = parsed.test_name {
        if use_nextest {
            cargo_args.push("-E".into());
            cargo_args.push(format!("test({name})"));
        } else {
            test_args.push(name.clone());
        }
    }

    (cargo_args, test_args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_schema_valid() {
        let tool = CargoTestTool::new(".");
        let params = tool.parameters();
        assert_eq!(params["type"], "object");
    }

    #[test]
    fn non_nextest_nocapture_and_test_name_single_separator() {
        let input = CargoTestInput {
            package: None,
            test_name: Some("my_test".into()),
            all_features: false,
            no_capture: true,
        };
        let (cargo_args, test_args) = build_args(&input, false);
        assert_eq!(cargo_args, vec!["test"]);
        assert_eq!(test_args, vec!["--nocapture", "my_test"]);
    }

    #[test]
    fn non_nextest_test_name_only() {
        let input = CargoTestInput {
            package: None,
            test_name: Some("my_test".into()),
            all_features: false,
            no_capture: false,
        };
        let (cargo_args, test_args) = build_args(&input, false);
        assert_eq!(cargo_args, vec!["test"]);
        assert_eq!(test_args, vec!["my_test"]);
    }

    #[test]
    fn nextest_test_name_uses_filter_expr() {
        let input = CargoTestInput {
            package: None,
            test_name: Some("my_test".into()),
            all_features: false,
            no_capture: false,
        };
        let (cargo_args, test_args) = build_args(&input, true);
        assert!(cargo_args.contains(&"-E".to_string()));
        assert!(cargo_args.contains(&"test(my_test)".to_string()));
        assert!(test_args.is_empty());
    }
}
