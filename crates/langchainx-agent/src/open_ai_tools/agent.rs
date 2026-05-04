use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use langchainx_chain::Chain;
use langchainx_core::{
    schemas::{
        agent::{AgentAction, AgentEvent, AgentFinish, LogTools},
        messages::Message,
    },
    tools::Tool,
};
use langchainx_llm::schemas::FunctionCallResponse;
use langchainx_prompt::{
    fmt_message, fmt_placeholder, fmt_template, message_formatter,
    prompt::{HumanMessagePromptTemplate, MessageFormatterStruct, PromptArgs},
    template_jinja2,
};

use crate::agent::Agent;
use crate::error::AgentError;

pub struct OpenAiToolAgent {
    pub(crate) chain: Box<dyn Chain>,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
}

impl OpenAiToolAgent {
    pub fn create_prompt(prefix: &str) -> Result<MessageFormatterStruct, AgentError> {
        let prompt = message_formatter![
            fmt_message!(Message::new_system_message(prefix)),
            fmt_placeholder!("chat_history"),
            fmt_template!(HumanMessagePromptTemplate::new(template_jinja2!(
                "{{input}}",
                "input"
            ))),
            fmt_placeholder!("agent_scratchpad")
        ];

        Ok(prompt)
    }

    fn construct_scratchpad(
        &self,
        intermediate_steps: &[(AgentAction, String)],
    ) -> Result<Vec<Message>, AgentError> {
        let mut thoughts: Vec<Message> = Vec::new();

        let mut tools_ai_message_seen: HashMap<String, ()> = HashMap::default();
        for (action, observation) in intermediate_steps {
            let LogTools { tool_id, tools } = serde_json::from_str(&action.log)?;
            let tools_vec: Vec<FunctionCallResponse> = serde_json::from_str(&tools)?;

            if tools_ai_message_seen.insert(tools, ()).is_none() {
                thoughts.push(Message::new_ai_message("").with_tool_calls(json!(tools_vec)));
            }

            thoughts.push(Message::new_tool_message(observation, tool_id));
        }

        Ok(thoughts)
    }
}

#[async_trait]
impl Agent for OpenAiToolAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let mut inputs = inputs.clone();
        let scratchpad = self.construct_scratchpad(intermediate_steps)?;
        inputs.insert("agent_scratchpad".to_string(), json!(scratchpad));
        let output = self.chain.call(inputs).await?.generation;
        match serde_json::from_str::<Vec<FunctionCallResponse>>(&output) {
            Ok(tools) => {
                let mut actions: Vec<AgentAction> = Vec::new();
                for tool in tools {
                    let log: LogTools = LogTools {
                        tool_id: tool.id.clone(),
                        tools: output.clone(),
                    };
                    actions.push(AgentAction {
                        tool: tool.function.name.clone(),
                        tool_input: tool.function.arguments.clone(),
                        log: serde_json::to_string(&log)?,
                    });
                }
                return Ok(AgentEvent::Action(actions));
            }
            Err(_) => return Ok(AgentEvent::Finish(AgentFinish { output })),
        }
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}
