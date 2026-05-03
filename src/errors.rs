use thiserror::Error;

use crate::{
    agent::AgentError, chain::ChainError, document_loaders::LoaderError, embedding::EmbedderError,
    language_models::LLMError, output_parsers::OutputParserError, prompt::PromptError,
    text_splitter::TextSplitterError, vectorstore::VectorStoreError,
};

#[derive(Error, Debug)]
pub enum LangChainError {
    #[error("LLM error: {0}")]
    LLM(#[from] LLMError),

    #[error("Chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("Agent error: {0}")]
    Agent(#[from] AgentError),

    #[error("Prompt error: {0}")]
    Prompt(#[from] PromptError),

    #[error("Output parser error: {0}")]
    OutputParser(#[from] OutputParserError),

    #[error("Document loader error: {0}")]
    Loader(#[from] LoaderError),

    #[error("Text splitter error: {0}")]
    TextSplitter(#[from] TextSplitterError),

    #[error("Embedder error: {0}")]
    Embedder(#[from] EmbedderError),

    #[error("Vector store error: {0}")]
    VectorStore(#[from] VectorStoreError),
}
