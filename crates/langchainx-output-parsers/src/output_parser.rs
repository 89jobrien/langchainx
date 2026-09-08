//! Common interface for transforming language-model output.
use async_trait::async_trait;

use super::OutputParserError;

#[async_trait]
/// Transforms raw model output into text expected by a caller.
pub trait OutputParser: Send + Sync {
    /// Parses or normalizes raw model output.
    async fn parse(&self, output: &str) -> Result<String, OutputParserError>;
}

impl<P> From<P> for Box<dyn OutputParser>
where
    P: OutputParser + 'static,
{
    fn from(parser: P) -> Self {
        Box::new(parser)
    }
}
