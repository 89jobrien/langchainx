use async_trait::async_trait;

use super::{OutputParser, OutputParserError};

pub struct SimpleParser {
    trim: bool,
}
impl SimpleParser {
    pub fn new() -> Self {
        Self { trim: false }
    }
    pub fn with_trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }
}
impl Default for SimpleParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OutputParser for SimpleParser {
    async fn parse(&self, output: &str) -> Result<String, OutputParserError> {
        if self.trim {
            Ok(output.trim().to_string())
        } else {
            Ok(output.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_parser_passthrough() {
        let parser = SimpleParser::new();
        let result = parser.parse("hello world").await.unwrap();
        assert_eq!(result, "hello world");
    }

    #[tokio::test]
    async fn test_simple_parser_preserves_whitespace_without_trim() {
        let parser = SimpleParser::new();
        let result = parser.parse("  hello  ").await.unwrap();
        assert_eq!(result, "  hello  ");
    }

    #[tokio::test]
    async fn test_simple_parser_trims_when_enabled() {
        let parser = SimpleParser::new().with_trim(true);
        let result = parser.parse("  hello  ").await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn test_simple_parser_empty_string() {
        let parser = SimpleParser::new();
        let result = parser.parse("").await.unwrap();
        assert_eq!(result, "");
    }
}
