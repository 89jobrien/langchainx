use thiserror::Error;

/// Top-level aggregate error for langchainx.
///
/// Each variant wraps a stringified module-level error. `#[from]` conversions
/// from typed errors (LLMError, ChainError, etc.) live in the root crate until
/// all modules are extracted into their own crates.
#[derive(Error, Debug)]
pub enum LangChainError {
    #[error("LLM error: {0}")]
    LLM(String),

    #[error("Chain error: {0}")]
    Chain(String),

    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Prompt error: {0}")]
    Prompt(String),

    #[error("Output parser error: {0}")]
    OutputParser(String),

    #[error("Document loader error: {0}")]
    Loader(String),

    #[error("Text splitter error: {0}")]
    TextSplitter(String),

    #[error("Embedder error: {0}")]
    Embedder(String),

    #[error("Vector store error: {0}")]
    VectorStore(String),
}
