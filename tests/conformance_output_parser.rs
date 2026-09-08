/// Conformance tests for the `OutputParser` trait contract.
///
/// Every `OutputParser` impl must satisfy:
/// 1. `parse()` with non-empty input returns Ok.
/// 2. `parse()` with empty input returns Ok (not an error).
/// 3. The parsed output is deterministic for the same input.
use langchainx::output_parsers::{OutputParser, SimpleParser};
use langchainx_testsuite::contracts::output_parser::{
    assert_deterministic_parse, assert_parse_contract,
};

#[tokio::test]
async fn simple_parser_parse_non_empty_returns_ok() {
    let parser = SimpleParser::new();
    assert_parse_contract(&parser, "hello world", "hello world").await;
}

#[tokio::test]
async fn simple_parser_parse_empty_returns_ok() {
    let parser = SimpleParser::new();
    assert_parse_contract(&parser, "", "").await;
}

#[tokio::test]
async fn simple_parser_is_deterministic() {
    let parser = SimpleParser::new();
    assert_deterministic_parse(&parser, "test input").await;
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
