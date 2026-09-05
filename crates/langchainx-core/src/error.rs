//! Top-level error aggregation for langchainx subsystems.
use thiserror::Error;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// Top-level error type for the langchainx library.
///
/// Each variant wraps a boxed dynamic error so that `langchainx-core` does not
/// depend on the higher-level crates that define subsystem-specific error types.
/// `From` implementations for the concrete subsystem error types are provided in
/// the root `langchainx` crate (`src/errors.rs`).
#[derive(Error, Debug)]
pub enum LangChainError {
    #[error("LLM error: {0}")]
    /// An error reported by a language model backend.
    LLM(BoxError),

    #[error("Chain error: {0}")]
    /// An error reported while running a chain.
    Chain(BoxError),

    #[error("Agent error: {0}")]
    /// An error reported while running an agent.
    Agent(BoxError),

    #[error("Prompt error: {0}")]
    /// An error reported while formatting a prompt.
    Prompt(BoxError),

    #[error("Output parser error: {0}")]
    /// An error reported while parsing model output.
    OutputParser(BoxError),

    #[error("Document loader error: {0}")]
    /// An error reported while loading documents.
    Loader(BoxError),

    #[error("Text splitter error: {0}")]
    /// An error reported while splitting text.
    TextSplitter(BoxError),

    #[error("Embedder error: {0}")]
    /// An error reported while generating embeddings.
    Embedder(BoxError),

    #[error("Vector store error: {0}")]
    /// An error reported by a vector store.
    VectorStore(BoxError),
}
