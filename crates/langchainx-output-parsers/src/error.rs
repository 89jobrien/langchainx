//! Errors produced while parsing language-model output.
use regex::Error as RegexError;
use thiserror::Error;

#[derive(Error, Debug)]
/// Errors produced while configuring or running an output parser.
pub enum OutputParserError {
    #[error("Regex error: {0}")]
    /// The parser's regular expression is invalid.
    RegexError(#[from] RegexError),

    #[error("Parsing error: {0}")]
    /// The output does not contain the expected structure.
    ParsingError(String),
}
