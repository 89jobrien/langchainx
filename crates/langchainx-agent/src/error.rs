//! Errors produced while building and running agents.
use thiserror::Error;

use langchainx_chain::ChainError;
use langchainx_llm::language_models::LLMError;
use langchainx_prompt::PromptError;

#[derive(Error, Debug)]
/// An error produced by agent construction, planning, or tool execution.
pub enum AgentError {
    #[error("LLM error: {0}")]
    /// The configured language model failed.
    LLMError(#[from] LLMError),

    #[error("Chain error: {0}")]
    /// An underlying chain failed.
    ChainError(#[from] ChainError),

    #[error("Prompt error: {0}")]
    /// Prompt formatting or rendering failed.
    PromptError(#[from] PromptError),

    #[error("Tool error: {0}")]
    /// A requested tool was unavailable or failed to run.
    ToolError(String),

    #[error("Missing Object On Builder: {0}")]
    /// A required builder component was not configured.
    MissingObject(String),

    #[error("Missing input variable: {0}")]
    /// An input required by the agent prompt was not provided.
    MissingInputVariable(String),

    #[error("Serde json error: {0}")]
    /// Agent state or model output contained invalid JSON.
    SerdeJsonError(#[from] serde_json::Error),

    #[error("Error: {0}")]
    /// An agent-specific failure not represented by another variant.
    OtherError(String),
}

impl From<AgentError> for langchainx_core::LangChainError {
    fn from(e: AgentError) -> Self {
        langchainx_core::LangChainError::Agent(Box::new(e))
    }
}
