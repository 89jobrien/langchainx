//! Traits and types for formatting text and chat prompts.
mod chat;
mod error;
#[allow(clippy::module_inception)]
mod prompt;

use std::collections::HashMap;

pub use chat::*;
pub use error::*;
pub use prompt::*;
use serde_json::Value;

use crate::schemas::{messages::Message, prompt::PromptValue};

/// Named JSON values supplied while formatting a prompt.
pub type PromptArgs = HashMap<String, Value>;

/// Formats a text template from named input values.
pub trait PromptFromatter: Send + Sync {
    /// Returns the unformatted template text.
    fn template(&self) -> String;
    /// Returns the variable names required by the template.
    fn variables(&self) -> Vec<String>;
    /// Substitutes input values into the template.
    fn format(&self, input_variables: PromptArgs) -> Result<String, PromptError>;
}
impl<PA> From<PA> for Box<dyn PromptFromatter>
where
    PA: PromptFromatter + 'static,
{
    fn from(prompt: PA) -> Self {
        Box::new(prompt)
    }
}

/// Formats named input values into chat messages.
pub trait MessageFormatter: Send + Sync {
    /// Produces the formatted messages.
    fn format_messages(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError>;
    /// Returns the input variable names required by the formatter.
    fn input_variables(&self) -> Vec<String>;
}
impl<MF> From<MF> for Box<dyn MessageFormatter>
where
    MF: MessageFormatter + 'static,
{
    fn from(prompt: MF) -> Self {
        Box::new(prompt)
    }
}

/// Formats named input values into a [`PromptValue`].
pub trait FormatPrompter: Send + Sync {
    /// Produces the formatted prompt value.
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError>;
    /// Returns the input variable names required by the prompt.
    fn get_input_variables(&self) -> Vec<String>;
}
impl<FP> From<FP> for Box<dyn FormatPrompter>
where
    FP: FormatPrompter + 'static,
{
    fn from(prompt: FP) -> Self {
        Box::new(prompt)
    }
}
