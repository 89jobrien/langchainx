use std::sync::Arc;

use tokio::sync::Mutex;

use crate::schemas::{memory::BaseMemory, messages::Message};

#[derive(Default)]
pub struct DummyMemory {}

impl DummyMemory {
    pub fn new() -> Self {
        Self {}
    }
}

impl From<DummyMemory> for Arc<Mutex<dyn BaseMemory>> {
    fn from(m: DummyMemory) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(m))
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
    use crate::schemas::memory::BaseMemory;
    use crate::schemas::messages::Message;

    #[test]
    fn messages_always_empty() {
        let mem = DummyMemory::new();
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn add_message_is_noop() {
        let mut mem = DummyMemory::new();
        mem.add_message(Message::new_human_message("anything"));
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn clear_is_noop() {
        let mut mem = DummyMemory::new();
        mem.add_message(Message::new_human_message("anything"));
        mem.clear();
        assert!(mem.messages().is_empty());
    }
}
