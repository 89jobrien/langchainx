//! Agent implementation that uses OpenAI-compatible native tool calls.
mod builder;
pub use builder::*;

mod agent;
pub use agent::*;

mod prompt;
