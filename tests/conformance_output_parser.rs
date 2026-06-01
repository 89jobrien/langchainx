/// Conformance tests for the `OutputParser` trait contract.
///
/// Every `OutputParser` impl must satisfy:
/// 1. `parse()` with non-empty input returns Ok.
/// 2. `parse()` with empty input returns Ok (not an error).
/// 3. The parsed output is deterministic for the same input.
use langchainx::output_parsers::{OutputParser, SimpleParser};

#[tokio::test]
async fn simple_parser_parse_non_empty_returns_ok() {
    let parser = SimpleParser::new();
    let result = parser.parse("hello world").await;
    assert!(result.is_ok(), "parse() must return Ok for valid input");
    assert_eq!(result.unwrap(), "hello world");
}

#[tokio::test]
async fn simple_parser_parse_empty_returns_ok() {
    let parser = SimpleParser::new();
    let result = parser.parse("").await;
    assert!(result.is_ok(), "parse('') must return Ok, not error");
    assert_eq!(result.unwrap(), "");
}

#[tokio::test]
async fn simple_parser_is_deterministic() {
    let parser = SimpleParser::new();
    let a = parser.parse("test input").await.unwrap();
    let b = parser.parse("test input").await.unwrap();
    assert_eq!(a, b, "same input must produce same output");
}

#[tokio::test]
async fn simple_parser_trim_mode_strips_whitespace() {
    let parser = SimpleParser::new().with_trim(true);
    let result = parser.parse("  trimmed  ").await.unwrap();
    assert_eq!(result, "trimmed");
}

#[tokio::test]
async fn simple_parser_passthrough_preserves_content() {
    let parser = SimpleParser::new();
    let input = "line1\nline2\n  indented";
    let result = parser.parse(input).await.unwrap();
    assert_eq!(
        result, input,
        "passthrough parser must preserve exact content"
    );
}
