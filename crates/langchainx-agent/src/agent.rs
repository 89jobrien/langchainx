use std::sync::Arc;

use async_trait::async_trait;

use langchainx_core::{
    schemas::agent::{AgentAction, AgentEvent},
    tools::Tool,
};
use langchainx_prompt::PromptArgs;

use crate::error::AgentError;

#[async_trait]
pub trait Agent: Send + Sync {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError>;

    fn get_tools(&self) -> Vec<Arc<dyn Tool>>;
}
