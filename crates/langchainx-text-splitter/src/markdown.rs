//! Markdown-aware token splitting.
use async_trait::async_trait;
use text_splitter::ChunkConfig;

use crate::{SplitterOptions, TextSplitter, TextSplitterError};

/// Splits Markdown while respecting its semantic structure where possible.
pub struct MarkdownSplitter {
    splitter_options: SplitterOptions,
}

impl Default for MarkdownSplitter {
    fn default() -> Self {
        MarkdownSplitter::new(SplitterOptions::default())
    }
}

impl MarkdownSplitter {
    /// Creates a Markdown splitter with the supplied options.
    pub fn new(options: SplitterOptions) -> Self {
        MarkdownSplitter {
            splitter_options: options,
        }
    }
}

#[async_trait]
impl TextSplitter for MarkdownSplitter {
    async fn split_text(&self, text: &str) -> Result<Vec<String>, TextSplitterError> {
        let chunk_config = ChunkConfig::try_from(&self.splitter_options)?;
        Ok(text_splitter::MarkdownSplitter::new(chunk_config)
            .chunks(text)
            .map(|x| x.to_string())
            .collect())
    }
}
