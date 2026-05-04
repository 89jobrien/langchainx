use std::sync::Arc;

use tokio::sync::Mutex;

use crate::schemas::{memory::BaseMemory, messages::Message};

#[derive(Default)]
pub struct SimpleMemory {
    messages: Vec<Message>,
}

impl SimpleMemory {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }
}

impl From<SimpleMemory> for Arc<Mutex<dyn BaseMemory>> {
    fn from(m: SimpleMemory) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(m))
    }
}

impl BaseMemory for SimpleMemory {
    fn messages(&self) -> Vec<Message> {
        self.messages.clone()
    }
    fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
    fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schemas::memory::BaseMemory;
    use crate::schemas::messages::{Message, MessageType};

    #[test]
    fn new_starts_empty() {
        let mem = SimpleMemory::new();
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn add_message_stores_in_order() {
        let mut mem = SimpleMemory::new();
        mem.add_message(Message::new_human_message("first"));
        mem.add_message(Message::new_ai_message("second"));
        let msgs = mem.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].content, "first");
        assert_eq!(msgs[1].content, "second");
    }

    #[test]
    fn messages_returns_all() {
        let mut mem = SimpleMemory::new();
        for i in 0..5 {
            mem.add_message(Message::new_human_message(i.to_string()));
        }
        assert_eq!(mem.messages().len(), 5);
    }

    #[test]
    fn clear_resets_to_empty() {
        let mut mem = SimpleMemory::new();
        mem.add_message(Message::new_human_message("hello"));
        mem.clear();
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn add_user_message_convenience() {
        let mut mem = SimpleMemory::new();
        mem.add_user_message(&"user input");
        let msgs = mem.messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message_type, MessageType::HumanMessage);
        assert_eq!(msgs[0].content, "user input");
    }

    #[test]
    fn add_ai_message_convenience() {
        let mut mem = SimpleMemory::new();
        mem.add_ai_message(&"ai response");
        let msgs = mem.messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].message_type, MessageType::AIMessage);
        assert_eq!(msgs[0].content, "ai response");
    }
}
