//! Wolfram Alpha API queries and plaintext pod extraction.
use serde_json::Value;

use crate::{Tool, ToolError};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframError {
    code: String,
    msg: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum WolframErrorStatus {
    Error(WolframError),
    NoError(bool),
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframResponse {
    queryresult: WolframResponseContent,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct WolframResponseContent {
    success: bool,
    error: WolframErrorStatus,
    pods: Option<Vec<Pod>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Pod {
    title: String,
    subpods: Vec<Subpod>,
}

impl From<Pod> for String {
    fn from(pod: Pod) -> String {
        let subpods_str: Vec<String> = pod
            .subpods
            .into_iter()
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();

        if subpods_str.is_empty() {
            return String::from("");
        }

        format!(
            "{{\"title\": {},\"subpods\": [{}]}}",
            pod.title,
            subpods_str.join(",")
        )
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Subpod {
    title: String,
    plaintext: String,
}

impl From<Subpod> for String {
    fn from(subpod: Subpod) -> String {
        if subpod.plaintext.is_empty() {
            return String::from("");
        }

        format!(
            "{{\"title\": \"{}\",\"plaintext\": \"{}\"}}",
            subpod.title,
            subpod.plaintext.replace("\n", " // ")
        )
    }
}

/// Queries Wolfram Alpha and returns non-empty plaintext result pods.
pub struct Wolfram {
    app_id: String,
    exclude_pods: Vec<String>,
    client: reqwest::Client,
}

impl Wolfram {
    /// Creates a Wolfram Alpha tool with an application ID.
    pub fn new(app_id: String) -> Self {
        Self {
            app_id,
            exclude_pods: Vec::new(),
            client: reqwest::Client::new(),
        }
    }

    /// Excludes result pods with the supplied identifiers.
    pub fn with_excludes<S: AsRef<str>>(mut self, exclude_pods: &[S]) -> Self {
        self.exclude_pods = exclude_pods.iter().map(|s| s.as_ref().to_owned()).collect();
        self
    }

    /// Replaces the Wolfram Alpha application ID.
    pub fn with_app_id<S: AsRef<str>>(mut self, app_id: S) -> Self {
        self.app_id = app_id.as_ref().to_owned();
        self
    }
}

impl Default for Wolfram {
    fn default() -> Wolfram {
        Wolfram {
            app_id: std::env::var("WOLFRAM_APP_ID").unwrap_or_default(),
            exclude_pods: Vec::new(),
            client: reqwest::Client::new(),
        }
    }
}

impl Tool for Wolfram {
    fn name(&self) -> String {
        String::from("Wolfram")
    }

    fn description(&self) -> String {
        String::from(
            "Wolfram Solver leverages the Wolfram Alpha computational engine
            to solve complex queries. Input should be a valid mathematical
            expression or query formulated in a way that Wolfram Alpha can
            interpret.",
        )
    }
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let input = input
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("input must be a string".to_string()))?;
        let mut url = format!(
            "https://api.wolframalpha.com/v2/query?appid={}&input={}&output=JSON&format=plaintext&podstate=Result__Step-by-step+solution",
            self.app_id,
            urlencoding::encode(input)
        );

        if !self.exclude_pods.is_empty() {
            url += &format!("&excludepodid={}", self.exclude_pods.join(","));
        }

        let response: WolframResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?
            .json()
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        if let WolframErrorStatus::Error(error) = response.queryresult.error {
            return Err(ToolError::ExecutionFailed(format!(
                "Wolfram Error {}: {}",
                error.code, error.msg
            )));
        } else if !response.queryresult.success {
            return Err(ToolError::ExecutionFailed(
                "Wolfram Error invalid query input: The query requested can not be processed by Wolfram".to_string(),
            ));
        }

        let pods_str: Vec<String> = response
            .queryresult
            .pods
            .unwrap_or_default()
            .into_iter()
            .map(String::from)
            .filter(|s| !s.is_empty())
            .collect();

        Ok(format!("{{\"pods\": [{}]}}", pods_str.join(",")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_wolfram() {
        let wolfram = Wolfram::default().with_excludes(&["Plot"]);
        let input = "Solve x^2 - 2x + 1 = 0";
        let result = wolfram.call(input).await;

        assert!(result.is_ok());
        println!("{}", result.unwrap());
    }

    #[test]
    fn tool_name_and_description_are_non_empty() {
        let tool = Wolfram::new("id".to_string());
        assert_eq!(tool.name(), "Wolfram");
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn builder_methods_set_fields() {
        // Verify the builder chain compiles and doesn't panic.
        let _tool = Wolfram::new("initial".to_string())
            .with_app_id("updated")
            .with_excludes(&["Plot", "Input"]);
    }

    #[tokio::test]
    async fn run_rejects_non_string_input() {
        let tool = Wolfram::new("dummy-id".to_string());
        let result = tool.run(serde_json::Value::Bool(true)).await;
        assert!(result.is_err());
    }

    #[test]
    fn subpod_empty_plaintext_converts_to_empty_string() {
        let subpod = Subpod {
            title: "t".to_string(),
            plaintext: String::new(),
        };
        let s = String::from(subpod);
        assert_eq!(s, "");
    }

    #[test]
    fn subpod_with_content_formats_json() {
        let subpod = Subpod {
            title: "Result".to_string(),
            plaintext: "x = 1".to_string(),
        };
        let s = String::from(subpod);
        assert!(s.contains("Result"));
        assert!(s.contains("x = 1"));
    }

    #[test]
    fn pod_with_all_empty_subpods_converts_to_empty_string() {
        let pod = Pod {
            title: "Empty".to_string(),
            subpods: vec![Subpod {
                title: String::new(),
                plaintext: String::new(),
            }],
        };
        let s = String::from(pod);
        assert_eq!(s, "");
    }

    #[test]
    fn pod_with_valid_subpod_formats_correctly() {
        let pod = Pod {
            title: "Solutions".to_string(),
            subpods: vec![Subpod {
                title: "x".to_string(),
                plaintext: "x = 1".to_string(),
            }],
        };
        let s = String::from(pod);
        assert!(s.contains("Solutions"));
        assert!(s.contains("x = 1"));
    }

    #[test]
    fn subpod_newline_is_replaced_by_separator() {
        let subpod = Subpod {
            title: "t".to_string(),
            plaintext: "line1\nline2".to_string(),
        };
        let s = String::from(subpod);
        assert!(s.contains("line1 // line2"));
        assert!(!s.contains('\n'));
    }
}
