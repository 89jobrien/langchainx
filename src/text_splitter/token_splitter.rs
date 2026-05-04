use async_trait::async_trait;
use text_splitter::ChunkConfig;
use tiktoken_rs::tokenizer::Tokenizer;

use super::{SplitterOptions, TextSplitter, TextSplitterError};

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
    pub fn new(options: SplitterOptions) -> TokenSplitter {
        TokenSplitter {
            splitter_options: options,
        }
    }

    #[deprecated = "Use `SplitterOptions::get_tokenizer_from_str` instead"]
    pub fn get_tokenizer_from_str(&self, s: &str) -> Option<Tokenizer> {
        match s.to_lowercase().as_str() {
            "cl100k_base" => Some(Tokenizer::Cl100kBase),
            "p50k_base" => Some(Tokenizer::P50kBase),
            "r50k_base" => Some(Tokenizer::R50kBase),
            "p50k_edit" => Some(Tokenizer::P50kEdit),
            "gpt2" => Some(Tokenizer::Gpt2),
            _ => None,
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
    async fn test_token_splitter_empty_input() {
        let splitter = TokenSplitter::default();
        let chunks = splitter.split_text("").await.unwrap();
        assert!(chunks.is_empty());
    }

    #[tokio::test]
    async fn test_token_splitter_produces_chunks() {
        let opts = SplitterOptions::new().with_chunk_size(10);
        let splitter = TokenSplitter::new(opts);
        let text = "The quick brown fox jumps over the lazy dog. ".repeat(5);
        let chunks = splitter.split_text(&text).await.unwrap();
        assert!(!chunks.is_empty());
    }

    #[tokio::test]
    async fn test_token_splitter_short_text_single_chunk() {
        let opts = SplitterOptions::new().with_chunk_size(512);
        let splitter = TokenSplitter::new(opts);
        let text = "Hello world.";
        let chunks = splitter.split_text(text).await.unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
    }

    #[tokio::test]
    async fn test_token_splitter_invalid_encoding_name_errors() {
        let opts = SplitterOptions::new()
            .with_encoding_name("not_a_real_encoding")
            .with_chunk_size(10);
        let splitter = TokenSplitter::new(opts);
        let result = splitter.split_text("some text").await;
        assert!(result.is_err());
    }
}
