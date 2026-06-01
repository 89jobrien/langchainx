/// Property tests — invariants that must hold for all valid inputs.
use proptest::prelude::*;

use langchainx::{
    prompt::{PromptFromatter, PromptTemplate, TemplateFormat},
    prompt_args,
    schemas::{Message, MessageType},
};

// ---------------------------------------------------------------------------
// Message serialization round-trip
// ---------------------------------------------------------------------------

fn arb_message_type() -> impl Strategy<Value = MessageType> {
    prop_oneof![
        Just(MessageType::HumanMessage),
        Just(MessageType::AIMessage),
        Just(MessageType::SystemMessage),
        Just(MessageType::ToolMessage),
    ]
}

proptest! {
    #[test]
    fn message_json_roundtrip(
        content in "\\PC{0,200}",
        msg_type in arb_message_type(),
    ) {
        let msg = Message {
            content: content.clone(),
            message_type: msg_type.clone(),
            ..Default::default()
        };
        let json = serde_json::to_string(&msg).unwrap();
        let roundtripped: Message = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(&roundtripped.content, &content);
        prop_assert_eq!(&roundtripped.message_type, &msg_type);
    }
}

// ---------------------------------------------------------------------------
// FString template formatting: all variables replaced, none left
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn fstring_template_replaces_all_variables(
        val_a in "[a-zA-Z0-9 ]{1,50}",
        val_b in "[a-zA-Z0-9 ]{1,50}",
    ) {
        let template = PromptTemplate::new(
            "Hello {name}, you are {age}".to_string(),
            vec!["name".to_string(), "age".to_string()],
            TemplateFormat::FString,
        );
        let args = prompt_args! {
            "name" => val_a.clone(),
            "age" => val_b.clone(),
        };
        let result = template.format(args).unwrap();
        // No unreplaced placeholders remain
        prop_assert!(!result.contains("{name}"), "unreplaced {{name}} in: {result}");
        prop_assert!(!result.contains("{age}"), "unreplaced {{age}} in: {result}");
        // Values appear in output
        prop_assert!(result.contains(&val_a), "{val_a} missing from: {result}");
        prop_assert!(result.contains(&val_b), "{val_b} missing from: {result}");
    }
}

// ---------------------------------------------------------------------------
// FString template: missing variable returns error
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn fstring_template_missing_var_is_err(
        val in "[a-zA-Z0-9]{1,30}",
    ) {
        let template = PromptTemplate::new(
            "Hello {name} and {other}".to_string(),
            vec!["name".to_string(), "other".to_string()],
            TemplateFormat::FString,
        );
        // Only provide one of two required variables
        let args = prompt_args! { "name" => val };
        let result = template.format(args);
        prop_assert!(result.is_err(), "should error on missing variable");
    }
}

// ---------------------------------------------------------------------------
// BaseMemory: SimpleMemory preserves insertion order for any messages
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn simple_memory_preserves_order(
        contents in prop::collection::vec("[a-zA-Z0-9 ]{1,50}", 1..20),
    ) {
        let mut mem = langchainx::memory::SimpleMemory::new();
        use langchainx::schemas::memory::BaseMemory;
        for c in &contents {
            mem.add_message(Message::new_human_message(c));
        }
        let msgs = mem.messages();
        prop_assert_eq!(msgs.len(), contents.len());
        for (msg, expected) in msgs.iter().zip(contents.iter()) {
            prop_assert_eq!(&msg.content, expected);
        }
    }
}

// ---------------------------------------------------------------------------
// WindowBufferMemory: never exceeds window size
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn window_buffer_never_exceeds_size(
        window_size in 1usize..20,
        msg_count in 0usize..50,
    ) {
        let mut mem = langchainx::memory::WindowBufferMemory::new(window_size);
        use langchainx::schemas::memory::BaseMemory;
        for i in 0..msg_count {
            mem.add_message(Message::new_human_message(format!("msg-{i}")));
        }
        let msgs = mem.messages();
        prop_assert!(
            msgs.len() <= window_size,
            "got {} messages with window_size={window_size}",
            msgs.len()
        );
        // If we added enough, buffer should be full
        if msg_count >= window_size {
            prop_assert_eq!(msgs.len(), window_size);
        }
    }
}

// ---------------------------------------------------------------------------
// WindowBufferMemory: retains the most recent messages
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(50))]
    #[test]
    fn window_buffer_retains_newest(
        window_size in 1usize..10,
        msg_count in 1usize..30,
    ) {
        let mut mem = langchainx::memory::WindowBufferMemory::new(window_size);
        use langchainx::schemas::memory::BaseMemory;
        let all_msgs: Vec<String> = (0..msg_count).map(|i| format!("msg-{i}")).collect();
        for m in &all_msgs {
            mem.add_message(Message::new_human_message(m));
        }
        let msgs = mem.messages();
        let expected_start = msg_count.saturating_sub(window_size);
        let expected: Vec<&str> = all_msgs[expected_start..].iter().map(|s| s.as_str()).collect();
        let actual: Vec<&str> = msgs.iter().map(|m| m.content.as_str()).collect();
        prop_assert_eq!(actual, expected);
    }
}

// ---------------------------------------------------------------------------
// TokenUsage::sum is commutative
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn token_usage_sum_commutative(
        p1 in 0u32..10000, c1 in 0u32..10000,
        p2 in 0u32..10000, c2 in 0u32..10000,
    ) {
        use langchainx::language_models::TokenUsage;
        let a = TokenUsage::new(p1, c1);
        let b = TokenUsage::new(p2, c2);
        let ab = a.sum(&b);
        let ba = b.sum(&a);
        prop_assert_eq!(ab.prompt_tokens, ba.prompt_tokens);
        prop_assert_eq!(ab.completion_tokens, ba.completion_tokens);
        prop_assert_eq!(ab.total_tokens, ba.total_tokens);
    }
}
