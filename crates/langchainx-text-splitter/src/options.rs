//! Shared configuration for token-aware and Markdown splitters.
use text_splitter::ChunkConfig;
use tiktoken_rs::{CoreBPE, get_bpe_from_model, get_bpe_from_tokenizer, tokenizer::Tokenizer};

use crate::TextSplitterError;

#[derive(Debug, Clone)]
/// Token-based chunking configuration.
pub struct SplitterOptions {
    /// Maximum tokens per chunk.
    pub chunk_size: usize,
    /// Tokens repeated between adjacent chunks.
    pub chunk_overlap: usize,
    /// Model used to resolve a tokenizer when `encoding_name` is empty.
    pub model_name: String,
    /// Explicit tokenizer encoding; takes precedence over `model_name` when non-empty.
    pub encoding_name: String,
    /// Whether to trim whitespace around emitted chunks.
    pub trim_chunks: bool,
}

impl Default for SplitterOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl SplitterOptions {
    /// Creates options with 512-token chunks, no overlap, and `cl100k_base` encoding.
    pub fn new() -> Self {
        SplitterOptions {
            chunk_size: 512,
            chunk_overlap: 0,
            model_name: String::from("gpt-3.5-turbo"),
            encoding_name: String::from("cl100k_base"),
            trim_chunks: false,
        }
    }

    /// Sets the maximum tokens per chunk.
    pub fn with_chunk_size(mut self, chunk_size: usize) -> Self {
        self.chunk_size = chunk_size;
        self
    }

    /// Sets the tokens repeated between adjacent chunks.
    pub fn with_chunk_overlap(mut self, chunk_overlap: usize) -> Self {
        self.chunk_overlap = chunk_overlap;
        self
    }

    /// Sets the model used when no encoding name is configured.
    pub fn with_model_name(mut self, model_name: &str) -> Self {
        self.model_name = String::from(model_name);
        self
    }

    /// Sets the explicit tokenizer encoding name.
    pub fn with_encoding_name(mut self, encoding_name: &str) -> Self {
        self.encoding_name = String::from(encoding_name);
        self
    }

    /// Configures whitespace trimming for emitted chunks.
    pub fn with_trim_chunks(mut self, trim_chunks: bool) -> Self {
        self.trim_chunks = trim_chunks;
        self
    }

    /// Resolves a supported tokenizer encoding name case-insensitively.
    pub fn get_tokenizer_from_str(s: &str) -> Option<Tokenizer> {
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

impl TryFrom<&SplitterOptions> for ChunkConfig<CoreBPE> {
    type Error = TextSplitterError;

    // qual:allow(iosp) reason: "I/O boundary — tokenizer resolution"
    fn try_from(options: &SplitterOptions) -> Result<Self, Self::Error> {
        let tk = if !options.encoding_name.is_empty() {
            let tokenizer = SplitterOptions::get_tokenizer_from_str(&options.encoding_name)
                .ok_or(TextSplitterError::TokenizerNotFound)?;
            get_bpe_from_tokenizer(tokenizer).map_err(|_| TextSplitterError::InvalidTokenizer)?
        } else {
            get_bpe_from_model(&options.model_name).map_err(|_| TextSplitterError::InvalidModel)?
        };

        Ok(ChunkConfig::new(options.chunk_size)
            .with_sizer(tk)
            .with_trim(options.trim_chunks)
            .with_overlap(options.chunk_overlap)?)
    }
}
