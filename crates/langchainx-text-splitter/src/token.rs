use async_trait::async_trait;
use text_splitter::ChunkConfig;

use crate::{SplitterOptions, TextSplitter, TextSplitterError};

#[derive(Debug, Clone)]
pub struct TokenSplitter {
    splitter_options: SplitterOptions,
}

impl Default for TokenSplitter {
    fn default() -> Self {
        TokenSplitter::new(SplitterOptions::default())
    }
}

impl TokenSplitter {
    pub fn new(options: SplitterOptions) -> Self {
        TokenSplitter {
            splitter_options: options,
        }
    }
}

#[async_trait]
impl TextSplitter for TokenSplitter {
    async fn split_text(&self, text: &str) -> Result<Vec<String>, TextSplitterError> {
        let chunk_config = ChunkConfig::try_from(&self.splitter_options)?;
        Ok(text_splitter::TextSplitter::new(chunk_config)
            .chunks(text)
            .map(|x| x.to_string())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_input() {
        let splitter = TokenSplitter::default();
        assert!(splitter.split_text("").await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn short_text_single_chunk() {
        let splitter = TokenSplitter::new(SplitterOptions::new().with_chunk_size(512));
        let chunks = splitter.split_text("Hello world.").await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "Hello world.");
    }

    #[tokio::test]
    async fn invalid_encoding_errors() {
        let splitter = TokenSplitter::new(
            SplitterOptions::new()
                .with_encoding_name("not_a_real_encoding")
                .with_chunk_size(10),
        );
        assert!(splitter.split_text("some text").await.is_err());
    }
}
