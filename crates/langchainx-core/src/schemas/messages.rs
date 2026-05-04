use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Enum `MessageType` represents the type of a message.
/// It can be a `SystemMessage`, `AIMessage`, or `HumanMessage`.
///
/// # Usage
/// ```rust,ignore
/// let system_message_type = MessageType::SystemMessage;
/// let ai_message_type = MessageType::AIMessage;
/// let human_message_type = MessageType::HumanMessage;
/// ```
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug, Clone, Default)]
pub enum MessageType {
    #[default]
    #[serde(rename = "system")]
    SystemMessage,
    #[serde(rename = "ai")]
    AIMessage,
    #[serde(rename = "human")]
    HumanMessage,
    #[serde(rename = "tool")]
    ToolMessage,
}

impl std::fmt::Display for MessageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            MessageType::SystemMessage => "system",
            MessageType::AIMessage => "ai",
            MessageType::HumanMessage => "human",
            MessageType::ToolMessage => "tool",
        };
        f.write_str(s)
    }
}

/// Struct `ImageContent` represents an image provided to an LLM.
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ImageContent {
    pub image_url: String,
    pub detail: Option<String>,
}

impl<S: AsRef<str>> From<S> for ImageContent {
    fn from(image_url: S) -> Self {
        ImageContent {
            image_url: image_url.as_ref().into(),
            detail: None,
        }
    }
}

/// Struct `Message` represents a message with its content and type.
///
/// # Usage
/// ```rust,ignore
/// let human_message = Message::new_human_message("Hello");
/// let system_message = Message::new_system_message("System Alert");
/// let ai_message = Message::new_ai_message("AI Response");
/// ```
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Message {
    pub content: String,
    pub message_type: MessageType,
    pub id: Option<String>,
    pub tool_calls: Option<Value>,
    pub images: Option<Vec<ImageContent>>,
}

impl Message {
    // Function to create a new Human message with a generic type that implements Display
    pub fn new_human_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: content.to_string(),
            message_type: MessageType::HumanMessage,
            id: None,
            tool_calls: None,
            images: None,
        }
    }

    pub fn new_human_message_with_images<T: Into<ImageContent>>(images: Vec<T>) -> Self {
        Message {
            content: String::default(),
            message_type: MessageType::HumanMessage,
            id: None,
            tool_calls: None,
            images: Some(images.into_iter().map(|i| i.into()).collect()),
        }
    }

    // Function to create a new System message with a generic type that implements Display
    pub fn new_system_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: content.to_string(),
            message_type: MessageType::SystemMessage,
            id: None,
            tool_calls: None,
            images: None,
        }
    }

    // Function to create a new AI message with a generic type that implements Display
    pub fn new_ai_message<T: std::fmt::Display>(content: T) -> Self {
        Message {
            content: content.to_string(),
            message_type: MessageType::AIMessage,
            id: None,
            tool_calls: None,
            images: None,
        }
    }

    // Function to create a new Tool message with a generic type that implements Display
    pub fn new_tool_message<T: std::fmt::Display, S: Into<String>>(content: T, id: S) -> Self {
        Message {
            content: content.to_string(),
            message_type: MessageType::ToolMessage,
            id: Some(id.into()),
            tool_calls: None,
            images: None,
        }
    }

    /// Sets the tool calls for the OpenAI-like API call.
    ///
    /// Use this method when you need to specify tool calls in the configuration.
    /// This is particularly useful in scenarios where interactions with specific
    /// tools are required for operation.
    ///
    /// # Arguments
    ///
    /// * `tool_calls` - A `serde_json::Value` representing the tool call configurations.
    pub fn with_tool_calls(mut self, tool_calls: Value) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    pub fn messages_from_value(value: &Value) -> Result<Vec<Message>, serde_json::error::Error> {
        serde_json::from_value(value.clone())
    }

    pub fn messages_to_string(messages: &[Message]) -> String {
        messages
            .iter()
            .map(|m| format!("{:?}: {}", m.message_type, m.content))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn human_message_type_and_content() {
        let m = Message::new_human_message("hello");
        assert_eq!(m.content, "hello");
        assert_eq!(m.message_type, MessageType::HumanMessage);
        assert!(m.id.is_none());
    }

    #[test]
    fn system_message_type_and_content() {
        let m = Message::new_system_message("sys");
        assert_eq!(m.message_type, MessageType::SystemMessage);
    }

    #[test]
    fn ai_message_type_and_content() {
        let m = Message::new_ai_message("response");
        assert_eq!(m.message_type, MessageType::AIMessage);
        assert_eq!(m.content, "response");
    }

    #[test]
    fn tool_message_has_id() {
        let m = Message::new_tool_message("result", "call-42");
        assert_eq!(m.message_type, MessageType::ToolMessage);
        assert_eq!(m.id.as_deref(), Some("call-42"));
    }

    #[test]
    fn with_tool_calls_sets_field() {
        let m = Message::new_ai_message("").with_tool_calls(json!({"fn": "foo"}));
        assert!(m.tool_calls.is_some());
    }

    #[test]
    fn messages_to_string_joins_with_newline() {
        let msgs = vec![
            Message::new_human_message("hi"),
            Message::new_ai_message("hello"),
        ];
        let s = Message::messages_to_string(&msgs);
        assert!(s.contains("hi"));
        assert!(s.contains("hello"));
        assert!(s.contains('\n'));
    }

    #[test]
    fn messages_from_value_round_trip() {
        let msgs = vec![Message::new_human_message("ping")];
        let v = serde_json::to_value(&msgs).unwrap();
        let restored = Message::messages_from_value(&v).unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].content, "ping");
        assert_eq!(restored[0].message_type, MessageType::HumanMessage);
    }

    #[test]
    fn message_type_to_string() {
        assert_eq!(MessageType::HumanMessage.to_string(), "human");
        assert_eq!(MessageType::AIMessage.to_string(), "ai");
        assert_eq!(MessageType::SystemMessage.to_string(), "system");
        assert_eq!(MessageType::ToolMessage.to_string(), "tool");
    }

    #[test]
    fn message_type_serde_round_trip() {
        let original = MessageType::ToolMessage;
        let json = serde_json::to_string(&original).unwrap();
        let restored: MessageType = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, original);
    }
}
