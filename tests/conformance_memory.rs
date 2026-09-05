use langchainx::schemas::Message;
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
use langchainx_testsuite::contracts::memory::{
    assert_memory_contract, assert_memory_format_contract, assert_noop_memory_contract,
};

#[test]
fn simple_memory_satisfies_contract() {
    assert_memory_contract(langchainx::memory::SimpleMemory::new());
    assert_memory_format_contract(
        langchainx::memory::SimpleMemory::new(),
        "human: hello\nai: hi there\nsystem: be helpful",
    );
}

#[test]
fn window_buffer_memory_satisfies_contract() {
    assert_memory_contract(langchainx::memory::WindowBufferMemory::new(100));
    assert_memory_format_contract(
        langchainx::memory::WindowBufferMemory::new(100),
        "human: hello\nai: hi there\nsystem: be helpful",
    );
}

#[test]
fn dummy_memory_satisfies_contract() {
    assert_noop_memory_contract(langchainx::memory::DummyMemory::new());
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
