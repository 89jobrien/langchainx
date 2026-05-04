use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;

use langchainx_chain::chain_trait::Chain;
use langchainx_core::{
    schemas::{
        agent::{AgentAction, AgentEvent},
        messages::Message,
    },
    tools::Tool,
};
use langchainx_prompt::{
    message_formatter, prompt_args, template_jinja2,
    prompt::{
        HumanMessagePromptTemplate, MessageFormatterStruct, MessageOrTemplate, PromptArgs,
        PromptFromatter,
    },
};

use crate::agent::Agent;
use crate::error::AgentError;

use super::{output_parser::ChatOutputParser, prompt::{FORMAT_INSTRUCTIONS, TEMPLATE_TOOL_RESPONSE}};

pub struct ConversationalAgent {
    pub(crate) chain: Box<dyn Chain>,
    pub(crate) tools: Vec<Arc<dyn Tool>>,
    pub(crate) output_parser: ChatOutputParser,
}

impl ConversationalAgent {
    pub fn create_prompt(
        tools: &[Arc<dyn Tool>],
        suffix: &str,
        prefix: &str,
    ) -> Result<MessageFormatterStruct, AgentError> {
        let tool_string = tools
            .iter()
            .map(|tool| format!("> {}: {}", tool.name(), tool.description()))
            .collect::<Vec<_>>()
            .join("\n");
        let tool_names = tools
            .iter()
            .map(|tool| tool.name())
            .collect::<Vec<_>>()
            .join(", ");

        let sufix_prompt = template_jinja2!(suffix, "tools", "format_instructions");

        let input_variables_fstring = prompt_args! {
            "tools" => tool_string,
            "format_instructions" => FORMAT_INSTRUCTIONS,
            "tool_names" => tool_names
        };

        let sufix_prompt = sufix_prompt.format(input_variables_fstring)?;
        let formatter = message_formatter![
            MessageOrTemplate::Message(Message::new_system_message(prefix)),
            MessageOrTemplate::MessagesPlaceholder("chat_history".to_string()),
            MessageOrTemplate::Template(
                HumanMessagePromptTemplate::new(template_jinja2!(
                    &sufix_prompt.to_string(),
                    "input"
                ))
                .into()
            ),
            MessageOrTemplate::MessagesPlaceholder("agent_scratchpad".to_string()),
        ];
        Ok(formatter)
    }

    fn construct_scratchpad(
        &self,
        intermediate_steps: &[(AgentAction, String)],
    ) -> Result<Vec<Message>, AgentError> {
        let mut thoughts: Vec<Message> = Vec::new();
        for (action, observation) in intermediate_steps.iter() {
            thoughts.push(Message::new_ai_message(&action.log));
            let tool_response = template_jinja2!(TEMPLATE_TOOL_RESPONSE, "observation")
                .format(prompt_args!("observation" => observation))?;
            thoughts.push(Message::new_human_message(&tool_response));
        }
        Ok(thoughts)
    }
}

#[async_trait]
impl Agent for ConversationalAgent {
    async fn plan(
        &self,
        intermediate_steps: &[(AgentAction, String)],
        inputs: PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let scratchpad = self.construct_scratchpad(intermediate_steps)?;
        let mut inputs = inputs.clone();
        inputs.insert("agent_scratchpad".to_string(), json!(scratchpad));
        let output = self.chain.call(inputs.clone()).await?.generation;
        let parsed_output = self.output_parser.parse(&output)?;
        Ok(parsed_output)
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}
