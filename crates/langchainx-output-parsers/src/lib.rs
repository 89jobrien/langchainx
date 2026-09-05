//! Parsers that normalize or extract structured text from model output.
#![deny(missing_docs)]
mod output_parser;
pub use output_parser::*;

mod markdown_parser;
pub use markdown_parser::*;

mod simple_parser;
pub use simple_parser::*;

mod error;
pub use error::*;
