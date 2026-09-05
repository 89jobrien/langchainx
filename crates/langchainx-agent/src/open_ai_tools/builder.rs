//! Builder for an OpenAI-compatible tool-calling agent.
use std::sync::Arc;

use langchainx_chain::{LLMChainBuilder, options::ChainCallOptions};
use langchainx_core::tools::Tool;
use langchainx_llm::{
    language_models::{llm::LLM, options::CallOptions},
    schemas::FunctionDefinition,
};

use crate::error::AgentError;

use super::{OpenAiToolAgent, prompt::PREFIX};

#[derive(Default)]
/// Configures tools, prompt text, and model options for an [`OpenAiToolAgent`].
pub struct OpenAiToolAgentBuilder {
    tools: Option<Vec<Arc<dyn Tool>>>,
    prefix: Option<String>,
    options: Option<ChainCallOptions>,
}

impl OpenAiToolAgentBuilder {
    /// Creates an empty builder that uses the default prompt and call options.
    pub fn new() -> Self {
        Self {
            tools: None,
            prefix: None,
            options: None,
        }
    }

    /// Sets the tools exposed as model function definitions.
    pub fn tools(mut self, tools: &[Arc<dyn Tool>]) -> Self {
        self.tools = Some(tools.to_vec());
        self
    }

    /// Replaces the system-message prefix.
    pub fn prefix<S: Into<String>>(mut self, prefix: S) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Sets the language-model call options.
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Builds an agent and registers each configured tool with `llm`.
    pub fn build<L: LLM + 'static>(self, llm: L) -> Result<OpenAiToolAgent, AgentError> {
        let tools = self.tools.unwrap_or_default();
        let prefix = self.prefix.unwrap_or_else(|| PREFIX.to_string());
        let mut llm = llm;

        let prompt = OpenAiToolAgent::create_prompt(&prefix)?;
        const DEFAULT_AGENT_MAX_TOKENS: u32 = 1000;
        let default_options = ChainCallOptions::default().with_max_tokens(DEFAULT_AGENT_MAX_TOKENS);
        let functions = tools
            .iter()
            .map(|tool| {
                FunctionDefinition::new(&tool.name(), &tool.description(), tool.parameters())
            })
            .collect::<Vec<FunctionDefinition>>();
        llm.add_options(CallOptions::new().with_functions(functions));
        let chain = Box::new(
            LLMChainBuilder::new()
                .prompt(prompt)
                .llm(llm)
                .options(self.options.unwrap_or(default_options))
                .build()?,
        );

        Ok(OpenAiToolAgent { chain, tools })
    }
}
