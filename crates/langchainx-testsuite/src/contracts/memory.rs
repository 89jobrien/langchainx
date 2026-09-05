use langchainx_core::schemas::{Message, MessageType, memory::BaseMemory};

/// Asserts storage, role helpers, clearing, and insertion-order behavior for mutable memory.
pub fn assert_memory_contract<M: BaseMemory>(mut memory: M) {
    assert!(memory.messages().is_empty(), "new memory must start empty");

    memory.add_message(Message::new_human_message("hello"));
    assert_eq!(memory.messages()[0].content, "hello");

    memory.clear();
    memory.add_user_message(&"user msg");
    assert_eq!(memory.messages()[0].message_type, MessageType::HumanMessage);

    memory.clear();
    memory.add_ai_message(&"ai msg");
    assert_eq!(memory.messages()[0].message_type, MessageType::AIMessage);

    memory.add_message(Message::new_human_message("leftover"));
    memory.clear();
    assert!(
        memory.messages().is_empty(),
        "clear must remove all messages"
    );

    memory.add_message(Message::new_human_message("first"));
    memory.add_message(Message::new_ai_message("second"));
    memory.add_message(Message::new_human_message("third"));
    let messages = memory.messages();
    let contents: Vec<&str> = messages
        .iter()
        .map(|message| message.content.as_str())
        .collect();
    assert_eq!(contents, ["first", "second", "third"]);
}

/// Asserts the weaker contract for memory implementations that deliberately discard messages.
pub fn assert_noop_memory_contract<M: BaseMemory>(mut memory: M) {
    assert!(memory.messages().is_empty());
    memory.add_message(Message::new_human_message("ignored"));
    assert!(memory.messages().is_empty());
    memory.clear();
    assert!(memory.messages().is_empty());
}

/// Adds representative message roles and asserts their exact newline-separated formatting.
pub fn assert_memory_format_contract<M: BaseMemory>(mut memory: M, expected: &str) {
    memory.add_user_message(&"hello");
    memory.add_ai_message(&"hi there");
    memory.add_message(Message::new_system_message("be helpful"));
    assert_eq!(memory.to_string(), expected);
}
