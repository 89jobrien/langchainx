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
    LLM(BoxError),

    #[error("Chain error: {0}")]
    Chain(BoxError),

    #[error("Agent error: {0}")]
    Agent(BoxError),

    #[error("Prompt error: {0}")]
    Prompt(BoxError),

    #[error("Output parser error: {0}")]
    OutputParser(BoxError),

    #[error("Document loader error: {0}")]
    Loader(BoxError),

    #[error("Text splitter error: {0}")]
    TextSplitter(BoxError),

    #[error("Embedder error: {0}")]
    Embedder(BoxError),

    #[error("Vector store error: {0}")]
    VectorStore(BoxError),
}
