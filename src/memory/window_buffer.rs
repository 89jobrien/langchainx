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

    #[test]
    fn oldest_message_evicted_when_window_full() {
        let window = 3;
        let mut mem = WindowBufferMemory::new(window);
        for i in 0..=window {
            mem.add_user_message(&format!("msg{}", i));
        }
        let msgs = mem.messages();
        assert_eq!(msgs.len(), window);
        // msg0 was evicted; first remaining is msg1
        assert_eq!(msgs[0].content, "msg1");
        assert_eq!(msgs[window - 1].content, format!("msg{}", window));
    }

    #[test]
    fn clear_resets_window_buffer() {
        let mut mem = WindowBufferMemory::new(3);
        mem.add_user_message(&"a");
        mem.add_user_message(&"b");
        mem.clear();
        assert!(mem.messages().is_empty());
        // After clear, new messages should be accepted normally
        mem.add_user_message(&"c");
        assert_eq!(mem.messages().len(), 1);
    }
}
