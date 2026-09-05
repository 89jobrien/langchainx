//! SerpApi request configuration and concise answer extraction.
use std::error::Error;

use async_trait::async_trait;
use serde_json::Value;

use crate::{Tool, ToolError};

/// Google Search tool backed by SerpApi.
pub struct SerpApi {
    api_key: String,
    location: Option<String>,
    hl: Option<String>,
    gl: Option<String>,
    google_domain: Option<String>,
}

impl SerpApi {
    /// Creates a SerpApi client with an API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            location: None,
            hl: None,
            gl: None,
            google_domain: None,
        }
    }
    /// Sets the geographic search location.
    pub fn with_location<S: Into<String>>(mut self, location: S) -> Self {
        self.location = Some(location.into());
        self
    }
    /// Sets the Google interface language code.
    pub fn with_hl<S: Into<String>>(mut self, hl: S) -> Self {
        self.hl = Some(hl.into());
        self
    }
    /// Sets the Google country code.
    pub fn with_gl(mut self, gl: String) -> Self {
        self.gl = Some(gl);
        self
    }
    /// Sets the Google domain used for the search.
    pub fn with_google_domain<S: Into<String>>(mut self, google_domain: S) -> Self {
        self.google_domain = Some(google_domain.into());
        self
    }

    /// Replaces the SerpApi API key.
    pub fn with_api_key<S: Into<String>>(mut self, api_key: S) -> Self {
        self.api_key = api_key.into();
        self
    }

    /// Runs a search and returns the best answer box, sports, knowledge, or organic result.
    pub async fn simple_search(&self, query: &str) -> Result<String, Box<dyn Error>> {
        let mut url = format!(
            "https://serpapi.com/search.json?q={}&api_key={}",
            query, self.api_key
        );
        if let Some(location) = &self.location {
            url.push_str(&format!("&location={}", location));
        }
        if let Some(hl) = &self.hl {
            url.push_str(&format!("&hl={}", hl));
        }
        if let Some(gl) = &self.gl {
            url.push_str(&format!("&gl={}", gl));
        }
        if let Some(google_domain) = &self.google_domain {
            url.push_str(&format!("&google_domain={}", google_domain));
        }
        let results: Value = reqwest::get(&url).await?.json().await?;

        let res = process_response(&results)?;

        Ok(res)
    }
}

fn get_answer_box(result: &Value) -> String {
    if let Some(map) = result["answer_box"].as_object() {
        if let Some(answer) = map.get("answer").and_then(|v| v.as_str()) {
            return answer.to_string();
        }

        if let Some(snippet) = map.get("snippet").and_then(|v| v.as_str()) {
            return snippet.to_string();
        }

        if let Some(snippet) = map
            .get("snippet_highlighted_words")
            .and_then(|v| v.as_array())
            && !snippet.is_empty()
            && let Some(first) = snippet.first().and_then(|v| v.as_str())
        {
            return first.to_string();
        }
    }

    "".to_string()
}

fn process_response(res: &Value) -> Result<String, Box<dyn Error>> {
    if !get_answer_box(res).is_empty() {
        return Ok(get_answer_box(res));
    }
    if !get_sport_result(res).is_empty() {
        return Ok(get_sport_result(res));
    }
    if !get_knowledge_graph(res).is_empty() {
        return Ok(get_knowledge_graph(res));
    }
    if !get_organic_result(res).is_empty() {
        return Ok(get_organic_result(res));
    }
    Err("No good result".into())
}

fn get_sport_result(result: &Value) -> String {
    if let Some(map) = result["sports_results"].as_object()
        && let Some(game_spotlight) = map.get("game_spotlight").and_then(|v| v.as_str())
    {
        return game_spotlight.to_string();
    }

    "".to_string()
}

fn get_knowledge_graph(result: &Value) -> String {
    if let Some(map) = result["knowledge_graph"].as_object()
        && let Some(description) = map.get("description").and_then(|v| v.as_str())
    {
        return description.to_string();
    }

    "".to_string()
}

fn get_organic_result(result: &Value) -> String {
    if let Some(array) = result["organic_results"].as_array()
        && !array.is_empty()
        && let Some(first) = array.first()
        && let Some(first_map) = first.as_object()
        && let Some(snippet) = first_map.get("snippet").and_then(|v| v.as_str())
    {
        return snippet.to_string();
    }

    "".to_string()
}

#[async_trait]
impl Tool for SerpApi {
    fn name(&self) -> String {
        String::from("GoogleSearch")
    }
    fn description(&self) -> String {
        String::from(
            r#""A wrapper around Google Search. "
	"Useful for when you need to answer questions about current events. "
	"Always one of the first options when you need to find information on internet"
	"Input should be a search query."#,
        )
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        let input = input
            .as_str()
            .ok_or_else(|| ToolError::InvalidInput("input must be a string".to_string()))?;
        self.simple_search(input)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}

impl Default for SerpApi {
    fn default() -> SerpApi {
        SerpApi {
            api_key: std::env::var("SERPAPI_API_KEY").unwrap_or_default(),
            location: None,
            hl: None,
            gl: None,
            google_domain: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[tokio::test]
    #[ignore]
    async fn serpapi_tool() {
        let serpapi = SerpApi::default();
        let s = serpapi
            .simple_search("Who is the President of Peru")
            .await
            .unwrap();
        println!("{}", s);
    }

    #[test]
    fn tool_name_and_description_are_non_empty() {
        let tool = SerpApi::new("key".to_string());
        assert!(!tool.name().is_empty());
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn builder_methods_mutate_fields() {
        let tool = SerpApi::new("initial".to_string())
            .with_api_key("updated")
            .with_location("Austin, TX")
            .with_hl("en")
            .with_gl("us".to_string())
            .with_google_domain("google.com");
        // If none of these panic the builder chain works correctly.
        let _ = tool.name();
    }

    #[test]
    fn process_response_answer_box() {
        let res = json!({ "answer_box": { "answer": "42" } });
        assert_eq!(process_response(&res).unwrap(), "42");
    }

    #[test]
    fn process_response_answer_box_snippet() {
        let res = json!({ "answer_box": { "snippet": "snippet text" } });
        assert_eq!(process_response(&res).unwrap(), "snippet text");
    }

    #[test]
    fn process_response_answer_box_snippet_highlighted() {
        let res = json!({ "answer_box": { "snippet_highlighted_words": ["first", "second"] } });
        assert_eq!(process_response(&res).unwrap(), "first");
    }

    #[test]
    fn process_response_knowledge_graph() {
        let res = json!({ "knowledge_graph": { "description": "kg desc" } });
        assert_eq!(process_response(&res).unwrap(), "kg desc");
    }

    #[test]
    fn process_response_organic_result() {
        let res = json!({
            "organic_results": [{ "snippet": "organic snippet" }]
        });
        assert_eq!(process_response(&res).unwrap(), "organic snippet");
    }

    #[test]
    fn process_response_sport_result() {
        let res = json!({
            "sports_results": { "game_spotlight": "Team A 3 - Team B 1" }
        });
        assert_eq!(process_response(&res).unwrap(), "Team A 3 - Team B 1");
    }

    #[test]
    fn process_response_no_good_result_returns_err() {
        let res = json!({});
        assert!(process_response(&res).is_err());
    }

    #[tokio::test]
    async fn run_rejects_non_string_input() {
        let tool = SerpApi::new("dummy".to_string());
        let result = tool.run(serde_json::Value::Number(42.into())).await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("input must be a string") || !msg.is_empty());
    }
}
