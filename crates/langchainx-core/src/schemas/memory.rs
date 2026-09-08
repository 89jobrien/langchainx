//! Conversation-memory abstraction shared by chains and agents.
use super::messages::Message;

/// Stores and formats messages from a conversation.
pub trait BaseMemory: Send + Sync {
    /// Returns the currently stored messages in conversation order.
    fn messages(&self) -> Vec<Message>;

    // Use a trait object for Display instead of a generic type
    /// Appends a human message created from a displayable value.
    fn add_user_message(&mut self, message: &dyn std::fmt::Display) {
        // Convert the Display trait object to a String and pass it to the constructor
        self.add_message(Message::new_human_message(message.to_string()));
    }

    // Use a trait object for Display instead of a generic type
    /// Appends an AI message created from a displayable value.
    fn add_ai_message(&mut self, message: &dyn std::fmt::Display) {
        // Convert the Display trait object to a String and pass it to the constructor
        self.add_message(Message::new_ai_message(message.to_string()));
    }

    /// Appends a message to memory.
    fn add_message(&mut self, message: Message);

    /// Removes all stored messages.
    fn clear(&mut self);

    /// Formats stored messages as newline-separated `role: content` lines.
    fn to_string(&self) -> String {
        self.messages()
            .iter()
            .map(|msg| format!("{}: {}", msg.message_type, msg.content))
            .collect::<Vec<String>>()
            .join("\n")
    }
}

impl<M> From<M> for Box<dyn BaseMemory>
where
    M: BaseMemory + 'static,
{
    fn from(memory: M) -> Self {
        Box::new(memory)
    }
}
