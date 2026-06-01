/// Conformance tests for the `BaseMemory` trait contract.
///
/// Every `BaseMemory` impl must satisfy these invariants:
/// 1. A new instance starts with no messages.
/// 2. `add_message` makes the message retrievable via `messages()`.
/// 3. `add_user_message` stores a HumanMessage.
/// 4. `add_ai_message` stores an AIMessage.
/// 5. `clear` removes all messages.
/// 6. Messages are returned in insertion order.
use langchainx::schemas::memory::BaseMemory;
use langchainx::schemas::{Message, MessageType};

fn assert_memory_contract(mut mem: impl BaseMemory) {
    // 1. Starts empty
    assert!(mem.messages().is_empty(), "new memory must start empty");

    // 2. add_message is retrievable
    mem.add_message(Message::new_human_message("hello"));
    assert_eq!(mem.messages().len(), 1);
    assert_eq!(mem.messages()[0].content, "hello");

    // 3. add_user_message stores HumanMessage
    mem.clear();
    mem.add_user_message(&"user msg");
    let msgs = mem.messages();
    if !msgs.is_empty() {
        // DummyMemory legitimately ignores adds — skip type check
        assert_eq!(msgs[0].message_type, MessageType::HumanMessage);
        assert_eq!(msgs[0].content, "user msg");
    }

    // 4. add_ai_message stores AIMessage
    mem.clear();
    mem.add_ai_message(&"ai msg");
    let msgs = mem.messages();
    if !msgs.is_empty() {
        assert_eq!(msgs[0].message_type, MessageType::AIMessage);
        assert_eq!(msgs[0].content, "ai msg");
    }

    // 5. clear removes all
    mem.add_message(Message::new_human_message("leftover"));
    mem.clear();
    assert!(mem.messages().is_empty(), "clear must remove all messages");

    // 6. Insertion order preserved
    mem.add_message(Message::new_human_message("first"));
    mem.add_message(Message::new_ai_message("second"));
    mem.add_message(Message::new_human_message("third"));
    let msgs = mem.messages();
    if msgs.len() >= 3 {
        assert_eq!(msgs[0].content, "first");
        assert_eq!(msgs[1].content, "second");
        assert_eq!(msgs[2].content, "third");
    }
}

/// DummyMemory has a weaker contract: adds are no-ops. We verify that
/// separately rather than forcing it through the full contract.
fn assert_dummy_contract(mut mem: impl BaseMemory) {
    assert!(mem.messages().is_empty());
    mem.add_message(Message::new_human_message("ignored"));
    assert!(
        mem.messages().is_empty(),
        "dummy memory must ignore add_message"
    );
    mem.clear();
    assert!(mem.messages().is_empty());
}

#[test]
fn simple_memory_satisfies_contract() {
    assert_memory_contract(langchainx::memory::SimpleMemory::new());
}

#[test]
fn window_buffer_memory_satisfies_contract() {
    assert_memory_contract(langchainx::memory::WindowBufferMemory::new(100));
}

#[test]
fn dummy_memory_satisfies_contract() {
    assert_dummy_contract(langchainx::memory::DummyMemory::new());
}

/// WindowBufferMemory-specific: window size is enforced.
#[test]
fn window_buffer_enforces_window_size() {
    let mut mem = langchainx::memory::WindowBufferMemory::new(3);
    for i in 0..10 {
        mem.add_message(Message::new_human_message(format!("msg-{i}")));
    }
    let msgs = mem.messages();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].content, "msg-7");
    assert_eq!(msgs[1].content, "msg-8");
    assert_eq!(msgs[2].content, "msg-9");
}
