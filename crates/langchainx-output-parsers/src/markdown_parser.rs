//! Extraction of content from fenced Markdown code blocks.
use async_trait::async_trait;
use regex::Regex;

use super::{OutputParser, OutputParserError};

/// Extracts the first capture group produced by a configurable regular expression.
pub struct MarkdownParser {
    expresion: String,
    trim: bool,
}
impl MarkdownParser {
    /// Creates a parser for the first fenced Markdown code block.
    pub fn new() -> Self {
        Self {
            expresion: r"```(?:\w+)?\s*([\s\S]+?)\s*```".to_string(),
            trim: false,
        }
    }

    /// Replaces the extraction regex, whose first capture group is returned.
    pub fn with_custom_expresion(mut self, expresion: &str) -> Self {
        self.expresion = expresion.to_string();
        self
    }

    /// Configures whether surrounding whitespace is removed from extracted text.
    pub fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }
}
impl Default for MarkdownParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutputParser for MarkdownParser {
    async fn parse(&self, output: &str) -> Result<String, OutputParserError> {
        let re = Regex::new(&self.expresion)?;
        if let Some(cap) = re.captures(output) {
            let find = cap
                .get(1)
                .ok_or_else(|| {
                    OutputParserError::ParsingError(
                        "Markdown parser expression must contain a capture group".into(),
                    )
                })?
                .as_str()
                .to_string();
            if self.trim {
                Ok(find.trim().to_string())
            } else {
                Ok(find)
            }
        } else {
            Err(OutputParserError::ParsingError(
                "No code block found".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_markdown_parser_no_code_block_returns_error() {
        let parser = MarkdownParser::new();
        let result = parser.parse("no code block here").await;
        assert!(result.is_err());
        match result.unwrap_err() {
            OutputParserError::ParsingError(msg) => assert!(msg.contains("No code block")),
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_markdown_parser_trim_code_block() {
        let parser = MarkdownParser::new().with_trim(true);
        let result = parser.parse("```\n  hello  \n```").await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_markdown_parser_unnamed_fence() {
        let parser = MarkdownParser::new();
        let result = parser.parse("```\nplain text\n```").await.unwrap();
        assert_eq!(result, "plain text");
    }

    #[tokio::test]
    async fn test_custom_expresion_is_used() {
        let parser = MarkdownParser::new().with_custom_expresion(r"<code>(.*?)</code>");
        let result = parser.parse("<code>hello</code>").await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn custom_expression_without_capture_group_returns_error() {
        let parser = MarkdownParser::new().with_custom_expresion(r"<code>.*?</code>");

        let result = parser.parse("<code>hello</code>").await;

        assert!(matches!(result, Err(OutputParserError::ParsingError(_))));
    }

    #[tokio::test]
    async fn test_default_expresion_extracts_code_block() {
        let parser = MarkdownParser::new();
        let result = parser.parse("```python\nprint('hi')\n```").await.unwrap();
        assert_eq!(result, "print('hi')");
    }

    #[tokio::test]
    async fn test_markdown_parser_finds_code_block() {
        let parser = MarkdownParser::new();
        let markdown_content = r#"
```rust
fn main() {
    println!("Hello, world!");
}
```
"#;
        let result = parser.parse(markdown_content).await;
        println!("{:?}", result);

        let correct = r#"fn main() {
    println!("Hello, world!");
}"#;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), correct);
    }
}
