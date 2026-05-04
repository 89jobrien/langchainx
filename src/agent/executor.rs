use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::Mutex;

use super::{agent::Agent, AgentError};
use crate::schemas::{LogTools, Message};
use crate::{
    chain::{chain_trait::Chain, ChainError},
    language_models::GenerateResult,
    memory::SimpleMemory,
    prompt::PromptArgs,
    schemas::{
        agent::{AgentAction, AgentEvent},
        memory::BaseMemory,
    },
    tools::Tool,
};

pub struct AgentExecutor<A>
where
    A: Agent,
{
    agent: A,
    max_iterations: Option<i32>,
    break_if_error: bool,
    pub memory: Option<Arc<Mutex<dyn BaseMemory>>>,
}

impl<A> AgentExecutor<A>
where
    A: Agent,
{
    pub fn from_agent(agent: A) -> Self {
        Self {
            agent,
            max_iterations: Some(10),
            break_if_error: false,
            memory: None,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: i32) -> Self {
        self.max_iterations = Some(max_iterations);
        self
    }

    pub fn with_memory(mut self, memory: Arc<Mutex<dyn BaseMemory>>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn with_break_if_error(mut self, break_if_error: bool) -> Self {
        self.break_if_error = break_if_error;
        self
    }

    fn get_name_to_tools(&self) -> HashMap<String, Arc<dyn Tool>> {
        let mut name_to_tool = HashMap::new();
        for tool in self.agent.get_tools().iter() {
            log::debug!("Loading Tool:{}", tool.name());
            name_to_tool.insert(tool.name().trim().replace(" ", "_"), tool.clone());
        }
        name_to_tool
    }
}

#[async_trait]
impl<A> Chain for AgentExecutor<A>
where
    A: Agent + Send + Sync,
{
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        let mut input_variables = input_variables.clone();
        let name_to_tools = self.get_name_to_tools();
        let mut steps: Vec<(AgentAction, String)> = Vec::new();
        log::debug!("steps: {:?}", steps);
        if let Some(memory) = &self.memory {
            let memory = memory.lock().await;
            input_variables.insert("chat_history".to_string(), json!(memory.messages()));
        } else {
            input_variables.insert(
                "chat_history".to_string(),
                json!(SimpleMemory::new().messages()),
            );
        }

        loop {
            let agent_event = self
                .agent
                .plan(&steps, input_variables.clone())
                .await
                .map_err(|e| ChainError::AgentError(format!("Error in agent planning: {}", e)))?;
            match agent_event {
                AgentEvent::Action(actions) => {
                    for action in actions {
                        log::debug!("Action: {:?}", action.tool_input);
                        let tool = name_to_tools
                            .get(&action.tool.trim().replace(" ", "_"))
                            .ok_or_else(|| {
                                AgentError::ToolError(format!("Tool {} not found", action.tool))
                            })
                            .map_err(|e| ChainError::AgentError(e.to_string()))?;

                        let observation_result = tool.call(&action.tool_input).await;

                        let observation = match observation_result {
                            Ok(result) => result,
                            Err(err) => {
                                log::info!(
                                    "The tool return the following error: {}",
                                    err.to_string()
                                );
                                if self.break_if_error {
                                    return Err(ChainError::AgentError(
                                        AgentError::ToolError(err.to_string()).to_string(),
                                    ));
                                } else {
                                    format!("The tool return the following error: {}", err)
                                }
                            }
                        };

                        steps.push((action, observation));
                    }
                }
                AgentEvent::Finish(finish) => {
                    if let Some(memory) = &self.memory {
                        let mut memory = memory.lock().await;

                        memory.add_user_message(match &input_variables["input"] {
                            // This avoids adding extra quotes to the user input in the history.
                            serde_json::Value::String(s) => s,
                            x => x, // this the json encoded value.
                        });

                        let mut tools_ai_message_seen: HashMap<String, ()> = HashMap::default();
                        for (action, observation) in steps {
                            let LogTools { tool_id, tools } = serde_json::from_str(&action.log)?;
                            let tools_value: serde_json::Value = serde_json::from_str(&tools)?;
                            if tools_ai_message_seen.insert(tools, ()).is_none() {
                                memory.add_message(
                                    Message::new_ai_message("").with_tool_calls(tools_value),
                                );
                            }
                            memory.add_message(Message::new_tool_message(observation, tool_id));
                        }

                        memory.add_ai_message(&finish.output);
                    }
                    return Ok(GenerateResult {
                        generation: finish.output,
                        ..Default::default()
                    });
                }
            }

            if let Some(max_iterations) = self.max_iterations {
                if steps.len() >= max_iterations as usize {
                    return Ok(GenerateResult {
                        generation: "Max iterations reached".to_string(),
                        ..Default::default()
                    });
                }
            }
        }
    }

    async fn invoke(&self, input_variables: PromptArgs) -> Result<String, ChainError> {
        let result = self.call(input_variables).await?;
        Ok(result.generation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent::AgentError,
        chain::chain_trait::Chain,
        prompt_args,
        schemas::agent::{AgentAction, AgentEvent, AgentFinish},
        tools::ToolError,
    };
    use async_trait::async_trait;
    use serde_json::Value;
    use std::sync::{Arc, Mutex as StdMutex};

    // ── FakeTool ────────────────────────────────────────────────────────────────

    struct FakeTool {
        name: String,
        response: String,
    }

    #[async_trait]
    impl Tool for FakeTool {
        fn name(&self) -> String {
            self.name.clone()
        }
        fn description(&self) -> String {
            "fake tool".into()
        }
        async fn run(&self, _input: Value) -> Result<String, ToolError> {
            Ok(self.response.clone())
        }
    }

    // ── FakeAgent ───────────────────────────────────────────────────────────────

    /// Plays back a pre-programmed sequence of AgentEvents, one per `plan()` call.
    struct FakeAgent {
        events: StdMutex<Vec<AgentEvent>>,
        tools: Vec<Arc<dyn Tool>>,
    }

    impl FakeAgent {
        fn new(mut events: Vec<AgentEvent>, tools: Vec<Arc<dyn Tool>>) -> Self {
            events.reverse(); // pop from back == consume from front
            Self {
                events: StdMutex::new(events),
                tools,
            }
        }
    }

    #[async_trait]
    impl Agent for FakeAgent {
        async fn plan(
            &self,
            _intermediate_steps: &[(AgentAction, String)],
            _inputs: PromptArgs,
        ) -> Result<AgentEvent, AgentError> {
            self.events
                .lock()
                .unwrap()
                .pop()
                .ok_or_else(|| AgentError::OtherError("no more events".into()))
        }

        fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
            self.tools.clone()
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────────────

    fn base_input() -> PromptArgs {
        prompt_args! { "input" => "test query" }
    }

    // ── tests ────────────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn single_tool_call_then_finish_returns_output() {
        let tool: Arc<dyn Tool> = Arc::new(FakeTool {
            name: "calculator".into(),
            response: "4".into(),
        });

        let events = vec![
            AgentEvent::Action(vec![AgentAction {
                tool: "calculator".into(),
                tool_input: "2+2".into(),
                log: "using calculator".into(),
            }]),
            AgentEvent::Finish(AgentFinish {
                output: "The answer is 4".into(),
            }),
        ];

        let agent = FakeAgent::new(events, vec![tool]);
        let executor = AgentExecutor::from_agent(agent);
        let result = executor.invoke(base_input()).await.unwrap();
        assert_eq!(result, "The answer is 4");
    }

    #[tokio::test]
    async fn max_iterations_exceeded_returns_sentinel_message() {
        let tool: Arc<dyn Tool> = Arc::new(FakeTool {
            name: "loop_tool".into(),
            response: "still going".into(),
        });

        // Provide more Action events than max_iterations allows.
        let events: Vec<AgentEvent> = (0..5)
            .map(|_| {
                AgentEvent::Action(vec![AgentAction {
                    tool: "loop_tool".into(),
                    tool_input: "x".into(),
                    log: "looping".into(),
                }])
            })
            .collect();

        let agent = FakeAgent::new(events, vec![tool]);
        let executor = AgentExecutor::from_agent(agent).with_max_iterations(2);
        let result = executor.invoke(base_input()).await.unwrap();
        assert_eq!(result, "Max iterations reached");
    }

    #[tokio::test]
    async fn unknown_tool_returns_chain_error() {
        let events = vec![AgentEvent::Action(vec![AgentAction {
            tool: "nonexistent_tool".into(),
            tool_input: "x".into(),
            log: "using nonexistent".into(),
        }])];

        let agent = FakeAgent::new(events, vec![]);
        let executor = AgentExecutor::from_agent(agent);
        let err = executor.invoke(base_input()).await.unwrap_err();
        match err {
            ChainError::AgentError(msg) => assert!(msg.contains("nonexistent_tool")),
            other => panic!("expected ChainError::AgentError, got {:?}", other),
        }
    }
}
