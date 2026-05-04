// LangChainError is defined in langchainx-core. Re-exported here for backwards
// compatibility. The From impls below convert typed module errors into the
// string-variant aggregate until all modules are extracted into crates.

pub use langchainx_core::errors::LangChainError;

use crate::{
    agent::AgentError, chain::ChainError, document_loaders::LoaderError, embedding::EmbedderError,
    language_models::LLMError, output_parsers::OutputParserError, prompt::PromptError,
    text_splitter::TextSplitterError, vectorstore::VectorStoreError,
};

impl From<LLMError> for LangChainError {
    fn from(e: LLMError) -> Self { LangChainError::LLM(e.to_string()) }
}

impl From<ChainError> for LangChainError {
    fn from(e: ChainError) -> Self { LangChainError::Chain(e.to_string()) }
}

impl From<AgentError> for LangChainError {
    fn from(e: AgentError) -> Self { LangChainError::Agent(e.to_string()) }
}

impl From<PromptError> for LangChainError {
    fn from(e: PromptError) -> Self { LangChainError::Prompt(e.to_string()) }
}

impl From<OutputParserError> for LangChainError {
    fn from(e: OutputParserError) -> Self { LangChainError::OutputParser(e.to_string()) }
}

impl From<LoaderError> for LangChainError {
    fn from(e: LoaderError) -> Self { LangChainError::Loader(e.to_string()) }
}

impl From<TextSplitterError> for LangChainError {
    fn from(e: TextSplitterError) -> Self { LangChainError::TextSplitter(e.to_string()) }
}

impl From<EmbedderError> for LangChainError {
    fn from(e: EmbedderError) -> Self { LangChainError::Embedder(e.to_string()) }
}

impl From<VectorStoreError> for LangChainError {
    fn from(e: VectorStoreError) -> Self { LangChainError::VectorStore(e.to_string()) }
}
