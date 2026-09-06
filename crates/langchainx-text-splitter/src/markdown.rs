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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input_produces_no_chunks() {
        let chunks = MarkdownSplitter::default().split_text("").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn short_markdown_preserves_structure_and_unicode() {
        let markdown = "# Caf\u{e9}\n\n```rust\nfn main() {}\n```";

        let chunks = MarkdownSplitter::default()
            .split_text(markdown)
            .await
            .unwrap();

        assert_eq!(chunks, vec![markdown]);
    }

    #[tokio::test]
    async fn chunking_without_overlap_preserves_all_content() {
        let markdown = (0..40)
            .map(|index| format!("## Section {index}\n\nParagraph {index}.\n\n"))
            .collect::<String>();
        let splitter = MarkdownSplitter::new(SplitterOptions::new().with_chunk_size(30));

        let chunks = splitter.split_text(&markdown).await.unwrap();

        assert!(chunks.len() > 1);
        assert_eq!(chunks.concat(), markdown);
    }

    #[tokio::test]
    async fn overlap_not_smaller_than_chunk_size_is_rejected() {
        let splitter = MarkdownSplitter::new(
            SplitterOptions::new()
                .with_chunk_size(8)
                .with_chunk_overlap(8),
        );

        assert!(splitter.split_text("# Heading\nBody").await.is_err());
    }
}
