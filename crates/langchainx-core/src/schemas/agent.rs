//! Values exchanged while an agent selects tools and produces a result.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Input supplied to a tool as text or named string values.
pub enum ToolInput {
    //Will implement this in the future
    /// A single string argument.
    StrInput(String),
    /// Named string arguments.
    DictInput(HashMap<String, String>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// A tool invocation requested by an agent.
pub struct AgentAction {
    /// Name of the tool to invoke.
    pub tool: String,
    /// Serialized input passed to the tool.
    pub tool_input: String, //this should be ToolInput in the future
    /// Agent output that led to this action.
    pub log: String,
}

/// Tool-call metadata emitted by OpenAI-compatible agents.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogTools {
    /// Provider-assigned identifier for the tool call.
    pub tool_id: String,
    /// Serialized tool-call payload.
    pub tools: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
/// Final output returned when an agent stops executing tools.
pub struct AgentFinish {
    /// Final response text.
    pub output: String,
}

#[derive(Debug)]
/// The next step produced by an agent.
pub enum AgentEvent {
    /// One or more tools should be invoked.
    Action(Vec<AgentAction>),
    /// Agent execution has completed.
    Finish(AgentFinish),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_action_fields() {
        let a = AgentAction {
            tool: "calculator".into(),
            tool_input: "2+2".into(),
            log: "using calculator".into(),
        };
        assert_eq!(a.tool, "calculator");
        assert_eq!(a.tool_input, "2+2");
    }

    #[test]
    fn agent_finish_field() {
        let f = AgentFinish {
            output: "42".into(),
        };
        assert_eq!(f.output, "42");
    }

    #[test]
    fn agent_action_serde_round_trip() {
        let a = AgentAction {
            tool: "search".into(),
            tool_input: "rust lang".into(),
            log: "searching".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let restored: AgentAction = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tool, "search");
        assert_eq!(restored.tool_input, "rust lang");
    }

    #[test]
    fn agent_finish_serde_round_trip() {
        let f = AgentFinish {
            output: "done".into(),
        };
        let json = serde_json::to_string(&f).unwrap();
        let restored: AgentFinish = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.output, "done");
    }
}
