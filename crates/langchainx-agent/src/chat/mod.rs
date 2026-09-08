//! ReAct-style conversational agent driven by JSON actions in chat responses.
mod builder;
mod chat_agent;
mod output_parser;
mod prompt;

pub use builder::*;
pub use chat_agent::*;
