use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub enum ToolInput {
    //Will implement this in the future
    StrInput(String),
    DictInput(HashMap<String, String>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentAction {
    pub tool: String,
    pub tool_input: String, //this should be ToolInput in the future
    pub log: String,
}

///Log tools is a struct used by the openai-like agents
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LogTools {
    pub tool_id: String,
    pub tools: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgentFinish {
    pub output: String,
}

#[derive(Debug)]
pub enum AgentEvent {
    Action(Vec<AgentAction>),
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
