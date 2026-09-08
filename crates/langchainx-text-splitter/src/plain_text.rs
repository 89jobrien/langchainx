//! Character-count-based plain-text splitting.
use async_trait::async_trait;

use crate::{TextSplitter, TextSplitterError};

#[derive(Debug, Clone)]
/// Character-based chunking configuration.
pub struct PlainTextSplitterOptions {
    /// Maximum characters per chunk.
    pub chunk_size: usize,
    /// Characters repeated between adjacent chunks.
    pub chunk_overlap: usize,
    /// Whether to trim whitespace around emitted chunks.
    pub trim_chunks: bool,
}

impl Default for PlainTextSplitterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl PlainTextSplitterOptions {
    /// Creates options with 512-character chunks, no overlap, and no trimming.
    pub fn new() -> Self {
        PlainTextSplitterOptions {
            chunk_size: 512,
            chunk_overlap: 0,
            trim_chunks: false,
        }
    }

    /// Sets the maximum characters per chunk.
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Sets the characters repeated between adjacent chunks.
    pub fn with_chunk_overlap(mut self, chunk_overlap: usize) -> Self {
        self.chunk_overlap = chunk_overlap;
        self
    }

    /// Configures whitespace trimming for emitted chunks.
    pub fn with_trim_chunks(mut self, trim_chunks: bool) -> Self {
        self.trim_chunks = trim_chunks;
        self
    }

    /// Returns the maximum characters per chunk.
    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    /// Returns the characters repeated between adjacent chunks.
    pub fn chunk_overlap(&self) -> usize {
        self.chunk_overlap
    }

    /// Returns whether emitted chunks are trimmed.
    pub fn trim_chunks(&self) -> bool {
        self.trim_chunks
    }
}

/// Splits plain text according to character-count limits.
pub struct PlainTextSplitter {
    splitter_options: PlainTextSplitterOptions,
}

impl Default for PlainTextSplitter {
    fn default() -> Self {
        PlainTextSplitter::new(PlainTextSplitterOptions::default())
    }
}

impl PlainTextSplitter {
    /// Creates a plain-text splitter with the supplied options.
    pub fn new(options: PlainTextSplitterOptions) -> Self {
        PlainTextSplitter {
            splitter_options: options,
        }
    }
}

#[async_trait]
impl TextSplitter for PlainTextSplitter {
    async fn split_text(&self, text: &str) -> Result<Vec<String>, TextSplitterError> {
        let splitter = text_splitter::TextSplitter::new(
            text_splitter::ChunkConfig::new(self.splitter_options.chunk_size)
                .with_trim(self.splitter_options.trim_chunks)
                .with_overlap(self.splitter_options.chunk_overlap)?,
        );
        Ok(splitter.chunks(text).map(|x| x.to_string()).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input() {
        let splitter = PlainTextSplitter::default();
        let chunks = splitter.split_text("").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn short_text_single_chunk() {
        let splitter = PlainTextSplitter::new(PlainTextSplitterOptions::new().with_chunk_size(512));
        let chunks = splitter.split_text("Short text.").await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Short text.");
    }

    #[tokio::test]
    async fn trim_chunks() {
        let splitter = PlainTextSplitter::new(
            PlainTextSplitterOptions::new()
                .with_chunk_size(50)
                .with_trim_chunks(true),
        );
        let chunks = splitter
            .split_text("  leading and trailing whitespace  ")
            .await
            .unwrap();
        for chunk in &chunks {
            assert_eq!(chunk.as_str(), chunk.trim());
        }
    }
}
