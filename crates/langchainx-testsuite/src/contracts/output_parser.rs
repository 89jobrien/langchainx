use langchainx_output_parsers::OutputParser;

/// Asserts that parsing succeeds and returns exactly the expected text.
pub async fn assert_parse_contract<P: OutputParser>(parser: &P, input: &str, expected: &str) {
    let result = parser
        .parse(input)
        .await
        .expect("OutputParser::parse must succeed");
    assert_eq!(result, expected);
}

/// Asserts that parsing identical input twice produces identical output.
pub async fn assert_deterministic_parse<P: OutputParser>(parser: &P, input: &str) {
    let first = parser.parse(input).await.expect("first parse must succeed");
    let second = parser
        .parse(input)
        .await
        .expect("second parse must succeed");
    assert_eq!(first, second);
}
