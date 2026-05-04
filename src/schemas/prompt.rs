use super::messages::Message;

#[derive(Debug, Clone)]
pub struct PromptValue {
    messages: Vec<Message>,
}
impl PromptValue {
    pub fn from_string(text: &str) -> Self {
        let message = Message::new_human_message(text);
        Self {
            messages: vec![message],
        }
    }
    pub fn from_messages(messages: Vec<Message>) -> Self {
        Self { messages }
    }

    pub fn to_string(&self) -> String {
        self.messages
            .iter()
            .map(|m| format!("{}: {}", m.message_type.to_string(), m.content))
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn to_chat_messages(&self) -> Vec<Message> {
        self.messages.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::messages::MessageType;

    #[test]
    fn from_string_creates_human_message() {
        let pv = PromptValue::from_string("hello");
        let msgs = pv.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "hello");
        assert_eq!(msgs[0].message_type, MessageType::HumanMessage);
    }

    #[test]
    fn from_messages_preserves_order() {
        let msgs = vec![
            Message::new_system_message("sys"),
            Message::new_human_message("hi"),
        ];
        let pv = PromptValue::from_messages(msgs.clone());
        let out = pv.to_chat_messages();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].message_type, MessageType::SystemMessage);
        assert_eq!(out[1].message_type, MessageType::HumanMessage);
    }

    #[test]
    fn to_string_contains_role_and_content() {
        let pv = PromptValue::from_messages(vec![
            Message::new_human_message("ping"),
            Message::new_ai_message("pong"),
        ]);
        let s = pv.to_string();
        assert!(s.contains("human"));
        assert!(s.contains("ping"));
        assert!(s.contains("ai"));
        assert!(s.contains("pong"));
    }
}
