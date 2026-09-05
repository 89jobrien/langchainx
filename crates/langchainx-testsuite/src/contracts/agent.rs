use langchainx_agent::Agent;
use langchainx_core::schemas::agent::AgentEvent;
use langchainx_prompt::PromptArgs;

/// Asserts that planning succeeds and returns an action or finish event.
pub async fn assert_plan_returns_event(agent: &dyn Agent, inputs: PromptArgs) -> AgentEvent {
    agent
        .plan(&[], inputs)
        .await
        .expect("Agent::plan must return an event")
}

/// Asserts that the agent exposes configured tools in the expected order.
pub fn assert_tool_names(agent: &dyn Agent, expected: &[&str]) {
    let actual: Vec<String> = agent
        .get_tools()
        .into_iter()
        .map(|tool| tool.name())
        .collect();
    assert_eq!(actual, expected, "Agent::get_tools returned wrong names");
}
