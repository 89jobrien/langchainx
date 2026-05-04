use std::sync::Arc;

use tokio::sync::Mutex;

use crate::schemas::{memory::BaseMemory, messages::Message};

pub struct WindowBufferMemory {
    window_size: usize,
    messages: Vec<Message>,
}

impl Default for WindowBufferMemory {
    fn default() -> Self {
        Self::new(10)
    }
}

impl WindowBufferMemory {
    pub fn new(window_size: usize) -> Self {
        Self {
            messages: Vec::new(),
            window_size,
        }
    }
}

impl Into<Arc<Mutex<dyn BaseMemory>>> for WindowBufferMemory {
    fn into(self) -> Arc<Mutex<dyn BaseMemory>> {
        Arc::new(Mutex::new(self))
    }
}

impl BaseMemory for WindowBufferMemory {
    fn messages(&self) -> Vec<Message> {
        self.messages.clone()
    }
    fn add_message(&mut self, message: Message) {
        if self.messages.len() >= self.window_size {
            self.messages.remove(0);
        }
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
    use crate::schemas::messages::Message;

    #[test]
    fn window_size_respected() {
        let mut mem = WindowBufferMemory::new(3);
        for i in 0..4 {
            mem.add_message(Message::new_human_message(i.to_string()));
        }
        assert_eq!(mem.messages().len(), 3);
    }

    #[test]
    fn oldest_messages_evicted() {
        let mut mem = WindowBufferMemory::new(2);
        mem.add_message(Message::new_human_message("first"));
        mem.add_message(Message::new_human_message("second"));
        mem.add_message(Message::new_human_message("third"));
        let msgs = mem.messages();
        assert_eq!(msgs[0].content, "second");
        assert_eq!(msgs[1].content, "third");
    }

    #[test]
    fn newest_messages_retained() {
        let mut mem = WindowBufferMemory::new(3);
        for i in 0..5u32 {
            mem.add_message(Message::new_human_message(i.to_string()));
        }
        let msgs = mem.messages();
        assert_eq!(msgs[0].content, "2");
        assert_eq!(msgs[1].content, "3");
        assert_eq!(msgs[2].content, "4");
    }

    #[test]
    fn clear_resets() {
        let mut mem = WindowBufferMemory::new(5);
        mem.add_message(Message::new_human_message("hello"));
        mem.clear();
        assert!(mem.messages().is_empty());
    }

    #[test]
    fn default_window_size_is_ten() {
        let mut mem = WindowBufferMemory::default();
        for i in 0..11u32 {
            mem.add_message(Message::new_human_message(i.to_string()));
        }
        assert_eq!(mem.messages().len(), 10);
    }
}
