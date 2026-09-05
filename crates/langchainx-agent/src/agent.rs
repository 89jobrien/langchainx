//! Core abstraction for agents that choose tools or finish with an answer.
use std::sync::Arc;

use async_trait::async_trait;

use langchainx_core::{
    schemas::agent::{AgentAction, AgentEvent},
    tools::Tool,
};
use langchainx_prompt::PromptArgs;

use crate::error::AgentError;

#[async_trait]
/// Plans the next action in an agent execution loop.
pub trait Agent: Send + Sync {
    /// Chooses tool actions or a final answer from the inputs and prior tool observations.
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError>;

    /// Returns the tools this agent may request.
    fn get_tools(&self) -> Vec<Arc<dyn Tool>>;
}
