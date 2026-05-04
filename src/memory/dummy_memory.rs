use std::sync::Arc;

use tokio::sync::Mutex;

use crate::schemas::{memory::BaseMemory, messages::Message};

pub struct DummyMemory {}

impl DummyMemory {
    pub fn new() -> Self {
        Self {}
    }
}

impl Into<Arc<Mutex<dyn BaseMemory>>> for DummyMemory {
    fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(self))
    }
}

impl BaseMemory for DummyMemory {
    fn messages(&self) -> Vec<Message> {
        vec![]
    }
    fn add_message(&mut self, _message: Message) {}
    fn clear(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_always_empty() {
        let mem = DummyMemory::new();
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn add_message_is_noop() {
        let mut mem = DummyMemory::new();
        mem.add_user_message(&"hello");
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn clear_is_noop() {
        let mut mem = DummyMemory::new();
        mem.clear();
        assert!(mem.messages().is_empty());
    }
}
