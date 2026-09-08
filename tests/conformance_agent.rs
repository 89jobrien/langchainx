/// Conformance tests for the `Agent` trait contract via `AgentExecutor`.
///
/// Every `Agent` impl must satisfy:
/// 1. `plan()` returns an AgentEvent (Action or Finish).
/// 2. `get_tools()` returns the tools the agent was configured with.
/// 3. `AgentExecutor::invoke()` drives the plan loop to completion.
/// 4. `AgentExecutor` respects max_iterations.
mod common;

use std::sync::Arc;

use common::{EchoTool, ScriptedAgent};
use langchainx::{
    agent::AgentExecutor,
    chain::Chain,
    prompt_args,
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::DynTool,
};
use langchainx_testsuite::contracts::agent::{
    assert_agent_contract, assert_executor_finishes, assert_executor_rejects_missing_input,
    assert_plan_returns_event, assert_tool_names,
};

#[tokio::test]
async fn agent_plan_returns_finish() {
    let agent = ScriptedAgent::finishing("done");
    let event =
        assert_plan_returns_event(&agent, langchainx::prompt_args! { "input" => "hi" }).await;
    match event {
        AgentEvent::Finish(finish) => assert_eq!(finish.output, "done"),
        AgentEvent::Action(_) => panic!("expected Finish, got Action"),
    }
}

#[tokio::test]
async fn agent_get_tools_returns_configured_tools() {
    let tool: Arc<dyn DynTool> = Arc::new(EchoTool);
    let agent = ScriptedAgent::new(vec![], vec![tool]);
    assert_tool_names(&agent, &["echo"]);
}

#[tokio::test]
async fn executor_drives_agent_to_finish() {
    let agent = ScriptedAgent::finishing("42");
    assert_executor_finishes(agent, prompt_args! { "input" => "compute" }, "42").await;
}

#[tokio::test]
async fn reusable_agent_contract_checks_plan_and_stable_tools() {
    let tool: Arc<dyn DynTool> = Arc::new(EchoTool);
    let agent = ScriptedAgent::new(
        vec![AgentEvent::Finish(AgentFinish {
            output: "done".into(),
        })],
        vec![tool],
    );
    let event = assert_agent_contract(&agent, prompt_args! { "input" => "go" }, &["echo"]).await;
    assert!(matches!(event, AgentEvent::Finish(_)));
}

#[tokio::test]
async fn executor_missing_input_contract_returns_typed_error() {
    assert_executor_rejects_missing_input(ScriptedAgent::finishing("unused")).await;
}

#[tokio::test]
async fn executor_calls_tool_then_finishes() {
    let tool: Arc<dyn DynTool> = Arc::new(EchoTool);
    let agent = ScriptedAgent::new(
        vec![
            AgentEvent::Action(vec![AgentAction {
                tool: "echo".into(),
                tool_input: "hello".into(),
                log: "".into(),
            }]),
            AgentEvent::Finish(AgentFinish {
                output: "done with tool".into(),
            }),
        ],
        vec![tool],
    );
    let executor = AgentExecutor::from_agent(agent);
    let result = executor
        .invoke(prompt_args! { "input" => "go" })
        .await
        .expect("executor invoke");
    assert_eq!(result, "done with tool");
}

#[tokio::test]
async fn executor_respects_max_iterations() {
    let tool: Arc<dyn DynTool> = Arc::new(EchoTool);
    let events: Vec<AgentEvent> = (0..20)
        .map(|_| {
            AgentEvent::Action(vec![AgentAction {
                tool: "echo".into(),
                tool_input: "x".into(),
                log: "".into(),
            }])
        })
        .collect();
    let agent = ScriptedAgent::new(events, vec![tool]);
    let executor = AgentExecutor::from_agent(agent).with_max_iterations(3);
    let result = executor
        .invoke(prompt_args! { "input" => "loop" })
        .await
        .expect("executor invoke");
    assert_eq!(result, "Max iterations reached");
}

#[tokio::test]
async fn executor_unknown_tool_returns_error() {
    let agent = ScriptedAgent::new(
        vec![AgentEvent::Action(vec![AgentAction {
            tool: "nonexistent".into(),
            tool_input: "x".into(),
            log: "".into(),
        }])],
        vec![], // no tools registered
    );
    let executor = AgentExecutor::from_agent(agent);
    let result = executor.invoke(prompt_args! { "input" => "test" }).await;
    assert!(
        result.is_err(),
        "executor must error when tool is not found"
    );
}
