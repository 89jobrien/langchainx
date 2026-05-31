/// Conformance tests for the `Agent` trait contract via `AgentExecutor`.
///
/// Every `Agent` impl must satisfy:
/// 1. `plan()` returns an AgentEvent (Action or Finish).
/// 2. `get_tools()` returns the tools the agent was configured with.
/// 3. `AgentExecutor::invoke()` drives the plan loop to completion.
/// 4. `AgentExecutor` respects max_iterations.
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::Mutex;

use langchainx::{
    agent::{Agent, AgentError, AgentExecutor},
    chain::Chain,
    prompt_args,
    schemas::agent::{AgentAction, AgentEvent, AgentFinish},
    tools::{Tool, ToolError},
};

struct FixedAgent {
    events: Arc<Mutex<Vec<AgentEvent>>>,
    tools: Vec<Arc<dyn Tool>>,
}

impl FixedAgent {
    fn finishing(output: &str) -> Self {
        Self {
            events: Arc::new(Mutex::new(vec![AgentEvent::Finish(AgentFinish {
                output: output.into(),
            })])),
            tools: vec![],
        }
    }

    fn with_events(events: Vec<AgentEvent>, tools: Vec<Arc<dyn Tool>>) -> Self {
        Self {
            events: Arc::new(Mutex::new(events)),
            tools,
        }
    }
}

#[async_trait]
impl Agent for FixedAgent {
    async fn plan(
        &self,
        _steps: &[(AgentAction, String)],
        _inputs: langchainx::prompt::PromptArgs,
    ) -> Result<AgentEvent, AgentError> {
        let mut events = self.events.lock().await;
        if events.is_empty() {
            Ok(AgentEvent::Finish(AgentFinish {
                output: "fallback".into(),
            }))
        } else {
            Ok(events.remove(0))
        }
    }

    fn get_tools(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.clone()
    }
}

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> String {
        "echo".into()
    }
    fn description(&self) -> String {
        "echoes input".into()
    }
    async fn run(&self, input: Value) -> Result<String, ToolError> {
        Ok(format!("echoed: {input}"))
    }
}

#[tokio::test]
async fn agent_plan_returns_finish() {
    let agent = FixedAgent::finishing("done");
    let event = agent
        .plan(&[], langchainx::prompt_args! { "input" => "hi" })
        .await
        .unwrap();
    match event {
        AgentEvent::Finish(f) => assert_eq!(f.output, "done"),
        AgentEvent::Action(_) => panic!("expected Finish, got Action"),
    }
}

#[tokio::test]
async fn agent_get_tools_returns_configured_tools() {
    let tool: Arc<dyn Tool> = Arc::new(EchoTool);
    let agent = FixedAgent::with_events(vec![], vec![tool.clone()]);
    let tools = agent.get_tools();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name(), "echo");
}

#[tokio::test]
async fn executor_drives_agent_to_finish() {
    let agent = FixedAgent::finishing("42");
    let executor = AgentExecutor::from_agent(agent);
    let result = executor
        .invoke(prompt_args! { "input" => "compute" })
        .await
        .expect("executor invoke");
    assert_eq!(result, "42");
}

#[tokio::test]
async fn executor_calls_tool_then_finishes() {
    let tool: Arc<dyn Tool> = Arc::new(EchoTool);
    let agent = FixedAgent::with_events(
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
    let tool: Arc<dyn Tool> = Arc::new(EchoTool);
    let events: Vec<AgentEvent> = (0..20)
        .map(|_| {
            AgentEvent::Action(vec![AgentAction {
                tool: "echo".into(),
                tool_input: "x".into(),
                log: "".into(),
            }])
        })
        .collect();
    let agent = FixedAgent::with_events(events, vec![tool]);
    let executor = AgentExecutor::from_agent(agent).with_max_iterations(3);
    let result = executor
        .invoke(prompt_args! { "input" => "loop" })
        .await
        .expect("executor invoke");
    assert_eq!(result, "Max iterations reached");
}
