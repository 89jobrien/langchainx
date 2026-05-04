use async_trait::async_trait;

use crate::{TextSplitter, TextSplitterError};

#[derive(Debug, Clone)]
pub struct PlainTextSplitterOptions {
    pub chunk_size: usize,
    pub chunk_overlap: usize,
    pub trim_chunks: bool,
}

impl Default for PlainTextSplitterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl PlainTextSplitterOptions {
    pub fn new() -> Self {
        PlainTextSplitterOptions {
            chunk_size: 512,
            chunk_overlap: 0,
            trim_chunks: false,
        }
    }

    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    pub fn with_chunk_overlap(mut self, chunk_overlap: usize) -> Self {
        self.chunk_overlap = chunk_overlap;
        self
    }

    pub fn with_trim_chunks(mut self, trim_chunks: bool) -> Self {
        self.trim_chunks = trim_chunks;
        self
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn chunk_overlap(&self) -> usize {
        self.chunk_overlap
    }

    pub fn trim_chunks(&self) -> bool {
        self.trim_chunks
    }
}

pub struct PlainTextSplitter {
    splitter_options: PlainTextSplitterOptions,
}

impl Default for PlainTextSplitter {
    fn default() -> Self {
        PlainTextSplitter::new(PlainTextSplitterOptions::default())
    }
}

impl PlainTextSplitter {
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
