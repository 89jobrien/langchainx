//! Chat-message templates and composite message formatters.
use crate::schemas::{messages::Message, prompt::PromptValue};

use super::{
    FormatPrompter, MessageFormatter, PromptArgs, PromptError, PromptFromatter, PromptTemplate,
};

/// Formats a [`PromptTemplate`] as a human message.
///
/// # Usage
/// ```rust,ignore
/// let human_message_prompt = HumanMessagePromptTemplate::new(template_fstring!(
///    "User says: {content}",
///    "content",
/// ));
/// ```
#[derive(Clone)]
pub struct HumanMessagePromptTemplate {
    prompt: PromptTemplate,
}

impl HumanMessagePromptTemplate {
    /// Wraps a text template that will produce human messages.
    pub fn new(prompt: PromptTemplate) -> Self {
        Self { prompt }
    }
}
impl MessageFormatter for HumanMessagePromptTemplate {
    fn format_messages(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError> {
        let message = Message::new_human_message(self.prompt.format(input_variables)?);
        log::debug!("message: {:?}", message);
        Ok(vec![message])
    }
    fn input_variables(&self) -> Vec<String> {
        self.prompt.variables().clone()
    }
}

impl FormatPrompter for HumanMessagePromptTemplate {
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError> {
        let messages = self.format_messages(input_variables)?;
        Ok(PromptValue::from_messages(messages))
    }
    fn get_input_variables(&self) -> Vec<String> {
        self.input_variables()
    }
}

/// Formats a [`PromptTemplate`] as a system message.
///
/// # Usage
/// ```rust,ignore
/// let system_message_prompt = SystemMessagePromptTemplate::new(template_fstring!(
///    "System alert: {alert_type} {alert_detail}",
///    "alert_type",
///    "alert_detail"
/// ));
/// ```
#[derive(Clone)]
pub struct SystemMessagePromptTemplate {
    prompt: PromptTemplate,
}

impl SystemMessagePromptTemplate {
    /// Wraps a text template that will produce system messages.
    pub fn new(prompt: PromptTemplate) -> Self {
        Self { prompt }
    }
}

impl FormatPrompter for SystemMessagePromptTemplate {
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError> {
        let messages = self.format_messages(input_variables)?;
        Ok(PromptValue::from_messages(messages))
    }
    fn get_input_variables(&self) -> Vec<String> {
        self.input_variables()
    }
}

impl MessageFormatter for SystemMessagePromptTemplate {
    fn format_messages(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError> {
        let message = Message::new_system_message(self.prompt.format(input_variables)?);
        log::debug!("message: {:?}", message);
        Ok(vec![message])
    }
    fn input_variables(&self) -> Vec<String> {
        self.prompt.variables().clone()
    }
}

/// Formats a [`PromptTemplate`] as an AI message.
///
/// # Usage
/// ```rust,ignore
/// let ai_message_prompt = AIMessagePromptTemplate::new(template_fstring!(
///    "AI response: {content} {additional_info}",
///    "content",
///    "additional_info"
/// ));
/// ```
#[derive(Clone)]
pub struct AIMessagePromptTemplate {
    prompt: PromptTemplate,
}

impl FormatPrompter for AIMessagePromptTemplate {
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError> {
        let messages = self.format_messages(input_variables)?;
        Ok(PromptValue::from_messages(messages))
    }
    fn get_input_variables(&self) -> Vec<String> {
        self.input_variables()
    }
}

impl MessageFormatter for AIMessagePromptTemplate {
    fn format_messages(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError> {
        let message = Message::new_ai_message(self.prompt.format(input_variables)?);
        log::debug!("message: {:?}", message);
        Ok(vec![message])
    }
    fn input_variables(&self) -> Vec<String> {
        self.prompt.variables().clone()
    }
}

impl AIMessagePromptTemplate {
    /// Wraps a text template that will produce AI messages.
    pub fn new(prompt: PromptTemplate) -> Self {
        Self { prompt }
    }
}

/// An item in a composite chat-message formatter.
pub enum MessageOrTemplate {
    /// A fixed message copied directly into the output.
    Message(Message),
    /// A formatter expanded using the supplied prompt arguments.
    Template(Box<dyn MessageFormatter>),
    /// A named argument containing serialized messages to insert.
    MessagesPlaceholder(String),
}

/// Wraps a fixed [`Message`](crate::schemas::messages::Message) for a composite formatter.
///
/// # Usage
/// The macro is called with a `Message` object. For example:
/// ```rust,ignore
/// let message = Message::new_human_message("Hello World");
/// fmt_message!(message) // Returns a `MessageOrTemplate::Message` variant that wraps the `Message` object
/// ```
#[macro_export]
macro_rules! fmt_message {
    ($msg:expr) => {
        $crate::prompt::MessageOrTemplate::Message($msg)
    };
}

/// Boxes a message formatter for inclusion in a composite formatter.
///
/// # Usage
/// The macro is called with a `MessageFormatter` object, for instance `HumanMessagePromptTemplate`,
/// `SystemMessagePromptTemplate`, `AIMessagePromptTemplate` or any other implementation of `MessageFormatter`.
///
/// ```rust,ignore
/// let prompt_template = HumanMessagePromptTemplate::new(template);
/// fmt_template!(prompt_template)
/// ```
#[macro_export]
macro_rules! fmt_template {
    ($template:expr) => {
        $crate::prompt::MessageOrTemplate::Template(Box::new($template))
    };
}

/// Creates a placeholder for a named list of serialized messages.
///
/// # Usage
/// The macro is called with a string literal or a String object:
/// ```rust,ignore
/// fmt_placeholder!("Placeholder message")
/// ```
#[macro_export]
macro_rules! fmt_placeholder {
    ($placeholder:expr) => {
        $crate::prompt::MessageOrTemplate::MessagesPlaceholder($placeholder.into())
    };
}

#[derive(Default)]
/// Builds an ordered prompt from fixed messages, templates, and placeholders.
pub struct MessageFormatterStruct {
    items: Vec<MessageOrTemplate>,
}

impl MessageFormatterStruct {
    /// Creates an empty message formatter.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Appends a fixed message.
    pub fn add_message(&mut self, message: Message) {
        self.items.push(MessageOrTemplate::Message(message));
    }

    /// Appends a message template.
    pub fn add_template(&mut self, template: Box<dyn MessageFormatter>) {
        self.items.push(MessageOrTemplate::Template(template));
    }

    /// Appends a placeholder resolved from prompt arguments during formatting.
    pub fn add_messages_placeholder(&mut self, placeholder: &str) {
        self.items.push(MessageOrTemplate::MessagesPlaceholder(
            placeholder.to_string(),
        ));
    }

    fn format(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError> {
        let mut result: Vec<Message> = Vec::new();
        for item in &self.items {
            match item {
                MessageOrTemplate::Message(msg) => result.push(msg.clone()),
                MessageOrTemplate::Template(tmpl) => {
                    result.extend(tmpl.format_messages(input_variables.clone())?)
                }
                MessageOrTemplate::MessagesPlaceholder(placeholder) => {
                    let messages = input_variables[placeholder].clone();
                    result.extend(Message::messages_from_value(&messages)?);
                }
            }
        }
        Ok(result)
    }
}

impl MessageFormatter for MessageFormatterStruct {
    fn format_messages(&self, input_variables: PromptArgs) -> Result<Vec<Message>, PromptError> {
        self.format(input_variables)
    }
    fn input_variables(&self) -> Vec<String> {
        let mut variables = Vec::new();
        for item in &self.items {
            match item {
                MessageOrTemplate::Message(_) => {}
                MessageOrTemplate::Template(tmpl) => {
                    variables.extend(tmpl.input_variables());
                }
                MessageOrTemplate::MessagesPlaceholder(placeholder) => {
                    variables.extend(vec![placeholder.clone()]);
                }
            }
        }
        variables
    }
}

impl FormatPrompter for MessageFormatterStruct {
    fn format_prompt(&self, input_variables: PromptArgs) -> Result<PromptValue, PromptError> {
        let messages = self.format(input_variables)?;
        Ok(PromptValue::from_messages(messages))
    }
    fn get_input_variables(&self) -> Vec<String> {
        self.input_variables()
    }
}

#[macro_export]
/// Creates a [`MessageFormatterStruct`](crate::prompt::MessageFormatterStruct) from ordered items.
///
///# Example
/// ```rust,ignore
/// // Create an AI message prompt template
/// let ai_message_prompt = AIMessagePromptTemplate::new(
/// template_fstring!(
///     "AI response: {content} {test}",
///     "content",
///     "test"
/// ));
///
/// let human_msg = Message::new_human_message("Hello from user");
///
/// // Use the `message_formatter` macro to construct the formatter.
/// let formatter = message_formatter![
///     fmt_message!(human_msg),
///     fmt_template!(ai_message_prompt),
///     fmt_placeholder!("history")
/// ];
/// ```
macro_rules! message_formatter {
($($item:expr),* $(,)?) => {{
    let mut formatter = $crate::prompt::MessageFormatterStruct::new();
    $(
        match $item {
            $crate::prompt::MessageOrTemplate::Message(msg) => formatter.add_message(msg),
            $crate::prompt::MessageOrTemplate::Template(tmpl) => formatter.add_template(tmpl),
            $crate::prompt::MessageOrTemplate::MessagesPlaceholder(placeholder) => formatter.add_messages_placeholder(&placeholder.clone()),
        }
    )*
    formatter
}};
}

#[cfg(test)]
mod tests {
    use crate::{
        prompt::{FormatPrompter, chat::AIMessagePromptTemplate},
        prompt_args,
        schemas::messages::Message,
        template_fstring,
    };

    #[test]
    fn test_message_formatter_macro() {
        // Create a human message and system message
        let human_msg = Message::new_human_message("Hello from user");

        // Create an AI message prompt template
        let ai_message_prompt = AIMessagePromptTemplate::new(template_fstring!(
            "AI response: {content} {test}",
            "content",
            "test"
        ));

        // Use the `message_formatter` macro to construct the formatter
        let formatter = message_formatter![
            fmt_message!(human_msg),
            fmt_template!(ai_message_prompt),
            fmt_placeholder!("history")
        ];

        // Define input variables for the AI message template
        let input_variables = prompt_args! {
            "content" => "This is a test",
            "test" => "test2",
            "history" => vec![
                Message::new_human_message("Placeholder message 1"),
                Message::new_ai_message("Placeholder message 2"),
            ],


        };

        // Format messages
        let formatted_messages = formatter
            .format_prompt(input_variables)
            .unwrap()
            .to_chat_messages();

        // Verify the number of messages
        assert_eq!(formatted_messages.len(), 4);

        // Verify the content of each message
        assert_eq!(formatted_messages[0].content, "Hello from user");
        assert_eq!(
            formatted_messages[1].content,
            "AI response: This is a test test2"
        );
        assert_eq!(formatted_messages[2].content, "Placeholder message 1");
        assert_eq!(formatted_messages[3].content, "Placeholder message 2");
    }
}
