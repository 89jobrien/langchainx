use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use langchainx_agent::{Agent, AgentError};
use langchainx_core::{
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::DynTool,
};
use langchainx_prompt::PromptArgs;
use tokio::sync::Mutex;

/// An agent that returns a predetermined sequence of actions and finishes.
#[derive(Clone)]
pub struct ScriptedAgent {
    events: Arc<Mutex<VecDeque<AgentEvent>>>,
    tools: Vec<Arc<dyn DynTool>>,
}

impl ScriptedAgent {
    /// Creates an agent with the supplied event sequence and available tools.
    pub fn new(events: Vec<AgentEvent>, tools: Vec<Arc<dyn DynTool>>) -> Self {
        Self {
            events: Arc::new(Mutex::new(events.into())),
            tools,
        }
    }

    /// Creates an agent that immediately finishes with `output`.
    pub fn finishing(output: impl Into<String>) -> Self {
        Self::new(
            vec![AgentEvent::Finish(AgentFinish {
                output: output.into(),
            })],
            Vec::new(),
        )
    }
}

#[async_trait]
impl Agent for ScriptedAgent {
    async fn plan(
        &self,
        _intermediate_steps: &[(AgentAction, String)],
        _inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        Ok(self.events.lock().await.pop_front().unwrap_or_else(|| {
            AgentEvent::Finish(AgentFinish {
                output: "fallback".into(),
            })
        }))
    }

    fn get_tools(&self) -> Vec<Arc<dyn DynTool>> {
        self.tools.clone()
    }
}
