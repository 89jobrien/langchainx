//! Builder for the ReAct-style conversational agent.
use std::sync::Arc;

use langchainx_chain::{llm_chain::LLMChainBuilder, options::ChainCallOptions};
use langchainx_core::tools::DynTool;
use langchainx_llm::language_models::llm::IntoArcLLM;

use crate::error::AgentError;

const DEFAULT_AGENT_MAX_TOKENS: u32 = 1000;

use super::{
    ConversationalAgent,
    output_parser::ChatOutputParser,
    prompt::{PREFIX, SUFFIX},
};

#[derive(Default)]
/// Configures the tools, prompt text, and model options for a [`ConversationalAgent`].
pub struct ConversationalAgentBuilder {
    tools: Option<Vec<Arc<dyn DynTool>>>,
    prefix: Option<String>,
    suffix: Option<String>,
    options: Option<ChainCallOptions>,
}

impl ConversationalAgentBuilder {
    /// Creates an empty builder that uses the default prompt and call options.
    pub fn new() -> Self {
        Self {
            tools: None,
            prefix: None,
            suffix: None,
            options: None,
        }
    }

    /// Sets the tools advertised to and callable by the agent.
    pub fn tools(mut self, tools: &[Arc<dyn DynTool>]) -> Self {
        self.tools = Some(tools.to_vec());
        self
    }

    /// Replaces the system-message prefix.
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Replaces the user-message template containing tool instructions.
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Sets the language-model call options.
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Builds an agent around `llm`, using a 1,000-token default when options are absent.
    pub fn build<L: IntoArcLLM>(self, llm: L) -> Result<ConversationalAgent, AgentError> {
        let tools = self.tools.unwrap_or_default();
        let prefix = self.prefix.unwrap_or_else(|| PREFIX.to_string());
        let suffix = self.suffix.unwrap_or_else(|| SUFFIX.to_string());

        let prompt = ConversationalAgent::create_prompt(&tools, &suffix, &prefix)?;
        let default_options = ChainCallOptions::default().with_max_tokens(DEFAULT_AGENT_MAX_TOKENS);
        let chain = Box::new(
            LLMChainBuilder::new()
                .prompt(prompt)
                .llm(llm)
                .options(self.options.unwrap_or(default_options))
                .build()?,
        );

        Ok(ConversationalAgent {
            chain,
            tools,
            output_parser: ChatOutputParser::new(),
        })
    }
}
