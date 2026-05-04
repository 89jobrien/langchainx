use std::collections::VecDeque;

use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

use langchainx_core::schemas::agent::{AgentAction, AgentEvent, AgentFinish};

use crate::error::AgentError;

use super::prompt::FORMAT_INSTRUCTIONS;

#[derive(Debug, Deserialize)]
struct AgentOutput {
    action: String,
    action_input: String,
}

pub struct ChatOutputParser {}

impl ChatOutputParser {
    pub fn new() -> Self {
        Self {}
    }
}

impl ChatOutputParser {
    pub fn parse(&self, text: &str) -> Result<AgentEvent, AgentError> {
        log::debug!("Parsing to Agent Action: {}", text);
        match parse_json_markdown(text) {
            Some(value) => {
                let agent_output: AgentOutput = serde_json::from_value(value)?;

                if agent_output.action == "Final Answer" {
                    Ok(AgentEvent::Finish(AgentFinish {
                        output: agent_output.action_input,
                    }))
                } else {
                    Ok(AgentEvent::Action(vec![AgentAction {
                        tool: agent_output.action,
                        tool_input: agent_output.action_input,
                        log: text.to_string(),
                    }]))
                }
            }
            None => {
                log::debug!("No JSON found or malformed JSON in text: {}", text);
                Ok(AgentEvent::Finish(AgentFinish {
                    output: text.to_string(),
                }))
            }
        }
    }

    #[allow(dead_code)]
    pub fn get_format_instructions(&self) -> &str {
        FORMAT_INSTRUCTIONS
    }
}

fn parse_partial_json(s: &str, strict: bool) -> Option<Value> {
    match serde_json::from_str::<Value>(s) {
        Ok(val) => return Some(val),
        Err(_) if !strict => (),
        Err(_) => return None,
    }

    let mut new_s = String::new();
    let mut stack: VecDeque<char> = VecDeque::new();
    let mut is_inside_string = false;
    let mut escaped = false;

    for char in s.chars() {
        match char {
            '"' if !escaped => is_inside_string = !is_inside_string,
            '{' if !is_inside_string => stack.push_back('}'),
            '[' if !is_inside_string => stack.push_back(']'),
            '}' | ']' if !is_inside_string => {
                if let Some(c) = stack.pop_back() {
                    if c != char {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            '\\' if is_inside_string => escaped = !escaped,
            _ => escaped = false,
        }
        new_s.push(char);
    }

    while let Some(c) = stack.pop_back() {
        new_s.push(c);
    }

    serde_json::from_str(&new_s).ok()
}

fn parse_json_markdown(json_markdown: &str) -> Option<Value> {
    let re = Regex::new(r"```(?:json)?\s*([\s\S]+?)\s*```").unwrap();
    re.captures(json_markdown)
        .and_then(|caps| caps.get(1))
        .and_then(|json_str| parse_partial_json(json_str.as_str(), false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parser() -> ChatOutputParser {
        ChatOutputParser::new()
    }

    #[test]
    fn parses_action_from_json_markdown() {
        let text = r#"```json
{"action": "calculator", "action_input": "2+2"}
```"#;
        let event = parser().parse(text).expect("parse failed");
        match event {
            AgentEvent::Action(actions) => {
                assert_eq!(actions.len(), 1);
                assert_eq!(actions[0].tool, "calculator");
                assert_eq!(actions[0].tool_input, "2+2");
            }
            AgentEvent::Finish(_) => panic!("expected Action, got Finish"),
        }
    }

    #[test]
    fn parses_final_answer_as_finish() {
        let text = r#"```json
{"action": "Final Answer", "action_input": "The answer is 42"}
```"#;
        let event = parser().parse(text).expect("parse failed");
        match event {
            AgentEvent::Finish(f) => assert_eq!(f.output, "The answer is 42"),
            AgentEvent::Action(_) => panic!("expected Finish, got Action"),
        }
    }

    #[test]
    fn no_json_block_returns_finish_with_raw_text() {
        let text = "The answer is 42";
        let event = parser().parse(text).expect("parse failed");
        match event {
            AgentEvent::Finish(f) => assert_eq!(f.output, "The answer is 42"),
            AgentEvent::Action(_) => panic!("expected Finish for plain text"),
        }
    }

    #[test]
    fn json_fenced_without_language_tag_parses() {
        let text = "```\n{\"action\": \"search\", \"action_input\": \"rust lang\"}\n```";
        let event = parser().parse(text).expect("parse failed");
        match event {
            AgentEvent::Action(actions) => assert_eq!(actions[0].tool, "search"),
            AgentEvent::Finish(_) => panic!("expected Action"),
        }
    }

    #[test]
    fn action_log_contains_original_text() {
        let text = "```json\n{\"action\": \"lookup\", \"action_input\": \"foo\"}\n```";
        let event = parser().parse(text).expect("parse failed");
        if let AgentEvent::Action(actions) = event {
            assert!(actions[0].log.contains("lookup"));
        }
    }

    #[test]
    fn missing_action_field_returns_error() {
        let text = "```json\n{\"foo\": \"bar\"}\n```";
        assert!(parser().parse(text).is_err());
    }
}
