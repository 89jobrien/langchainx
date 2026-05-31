/// Conformance tests for the prompt layer contracts.
///
/// PromptFromatter contract:
/// 1. `template()` returns the raw template string.
/// 2. `variables()` returns declared variable names.
/// 3. `format()` substitutes variables into the template.
/// 4. `format()` with missing variables returns Err.
///
/// FormatPrompter contract:
/// 1. `format_prompt()` returns a PromptValue with messages.
/// 2. `get_input_variables()` matches the template variables.
///
/// MessageFormatter contract:
/// 1. `format_messages()` returns correctly typed messages.
/// 2. `input_variables()` lists the required variables.
use langchainx::{
    prompt::{
        FormatPrompter, HumanMessagePromptTemplate, MessageFormatter,
        PromptFromatter, PromptTemplate, TemplateFormat,
    },
    prompt_args,
    schemas::MessageType,
    template_fstring,
};

// -- PromptFromatter --

#[test]
fn prompt_formatter_template_returns_raw_string() {
    let pt = template_fstring!("Hello {name}, welcome to {place}!", "name", "place");
    assert!(pt.template().contains("{name}"));
    assert!(pt.template().contains("{place}"));
}

#[test]
fn prompt_formatter_variables_lists_declared_vars() {
    let pt = template_fstring!("Q: {question}", "question");
    let vars = pt.variables();
    assert_eq!(vars, vec!["question"]);
}

#[test]
fn prompt_formatter_format_substitutes_variables() {
    let pt = template_fstring!("{greeting} {who}!", "greeting", "who");
    let result = pt
        .format(prompt_args! { "greeting" => "Hi", "who" => "world" })
        .unwrap();
    assert_eq!(result, "Hi world!");
}

#[test]
fn prompt_formatter_format_missing_var_returns_err() {
    let pt = template_fstring!("{a} and {b}", "a", "b");
    let result = pt.format(prompt_args! { "a" => "alpha" });
    assert!(
        result.is_err(),
        "format() with missing variable must return Err"
    );
}

// -- FormatPrompter --

#[test]
fn format_prompter_returns_prompt_value_with_messages() {
    let pt = HumanMessagePromptTemplate::new(template_fstring!("Say {word}", "word"));
    let pv = pt
        .format_prompt(prompt_args! { "word" => "hello" })
        .unwrap();
    let msgs = pv.to_chat_messages();
    assert!(!msgs.is_empty(), "PromptValue must contain messages");
    assert_eq!(msgs[0].content, "Say hello");
}

#[test]
fn format_prompter_get_input_variables_matches_template() {
    let pt = HumanMessagePromptTemplate::new(template_fstring!(
        "{a} {b} {c}",
        "a",
        "b",
        "c"
    ));
    let vars = pt.get_input_variables();
    assert_eq!(vars.len(), 3);
    assert!(vars.contains(&"a".to_string()));
    assert!(vars.contains(&"b".to_string()));
    assert!(vars.contains(&"c".to_string()));
}

// -- MessageFormatter --

#[test]
fn message_formatter_produces_human_message() {
    let hm = HumanMessagePromptTemplate::new(template_fstring!("User: {msg}", "msg"));
    let msgs = hm
        .format_messages(prompt_args! { "msg" => "hi" })
        .unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].message_type, MessageType::HumanMessage);
    assert_eq!(msgs[0].content, "User: hi");
}

#[test]
fn message_formatter_input_variables_matches() {
    let hm = HumanMessagePromptTemplate::new(template_fstring!("{x}", "x"));
    let vars = hm.input_variables();
    assert_eq!(vars, vec!["x"]);
}

// -- PromptTemplate as FormatPrompter --

#[test]
fn prompt_template_format_prompt_wraps_as_human_message() {
    let pt = PromptTemplate::new(
        "Tell me about {topic}".into(),
        vec!["topic".into()],
        TemplateFormat::FString,
    );
    let pv = pt
        .format_prompt(prompt_args! { "topic" => "Rust" })
        .unwrap();
    let msgs = pv.to_chat_messages();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].message_type, MessageType::HumanMessage);
    assert!(msgs[0].content.contains("Rust"));
}
