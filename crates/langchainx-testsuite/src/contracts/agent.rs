use langchainx_agent::{Agent, AgentExecutor};
use langchainx_chain::{Chain, ChainError};
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
        .map(|tool| tool.dyn_name())
        .collect();
    assert_eq!(actual, expected, "Agent::get_tools returned wrong names");
}

/// Verifies that planning succeeds and tool registration is stable across calls.
pub async fn assert_agent_contract(
    agent: &dyn Agent,
    inputs: PromptArgs,
    expected_tools: &[&str],
) -> AgentEvent {
    assert_tool_names(agent, expected_tools);
    assert_tool_names(agent, expected_tools);
    assert_plan_returns_event(agent, inputs).await
}

/// Verifies that an executor reaches the expected final generation.
pub async fn assert_executor_finishes<A>(agent: A, inputs: PromptArgs, expected: &str)
where
    A: Agent,
{
    let generation = AgentExecutor::from_agent(agent)
        .invoke(inputs)
        .await
        .expect("AgentExecutor must reach a finish event");
    assert_eq!(
        generation, expected,
        "executor returned the wrong generation"
    );
}

/// Verifies that an executor returns a typed missing-input error instead of panicking.
pub async fn assert_executor_rejects_missing_input<A>(agent: A)
where
    A: Agent,
{
    let result = AgentExecutor::from_agent(agent)
        .invoke(PromptArgs::new())
        .await;
    assert!(
        matches!(result, Err(ChainError::MissingInputVariable { ref key, .. }) if key == "input"),
        "executor must reject a missing input key"
    );
}
