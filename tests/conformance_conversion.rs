//! Conformance tests for bidirectional OpenAI-compatible schema conversions.

use async_openai::types::{
    ChatCompletionTool, ChatCompletionToolType, FunctionObject,
    ResponseFormat as OpenAiResponseFormat,
};
use langchainx_core::schemas::convert::TryOpenAiIntoLangchain;
use langchainx_llm::schemas::{FunctionCallBehavior, FunctionDefinition, ResponseFormat};
use langchainx_testsuite::contracts::conversion::{
    assert_openai_round_trip, assert_openai_try_round_trip,
};
use serde_json::json;

#[test]
fn function_call_behavior_round_trips_every_openai_variant() {
    assert_openai_round_trip::<_, async_openai::types::ChatCompletionToolChoiceOption>(
        FunctionCallBehavior::None,
    );
    assert_openai_round_trip::<_, async_openai::types::ChatCompletionToolChoiceOption>(
        FunctionCallBehavior::Auto,
    );
    assert_openai_round_trip::<_, async_openai::types::ChatCompletionToolChoiceOption>(
        FunctionCallBehavior::Required,
    );
    assert_openai_round_trip::<_, async_openai::types::ChatCompletionToolChoiceOption>(
        FunctionCallBehavior::Named("lookup_weather".to_owned()),
    );
}

#[test]
fn response_format_round_trips_every_openai_variant() {
    assert_openai_round_trip::<_, OpenAiResponseFormat>(ResponseFormat::Text);
    assert_openai_round_trip::<_, OpenAiResponseFormat>(ResponseFormat::JsonObject);
    assert_openai_round_trip::<_, OpenAiResponseFormat>(ResponseFormat::JsonSchema {
        description: Some("A weather report".to_owned()),
        name: "weather_report".to_owned(),
        schema: Some(json!({"type": "object"})),
        strict: Some(true),
    });
}

#[test]
fn function_definition_round_trips_when_required_fields_are_present() {
    assert_openai_try_round_trip::<_, ChatCompletionTool>(FunctionDefinition::new(
        "lookup weather",
        "Looks up weather by city",
        json!({
            "type": "object",
            "properties": {"city": {"type": "string"}},
            "required": ["city"]
        }),
    ));
}

#[test]
fn function_definition_reverse_conversion_rejects_missing_description() {
    let openai = ChatCompletionTool {
        r#type: ChatCompletionToolType::Function,
        function: FunctionObject {
            name: "lookup_weather".to_owned(),
            description: None,
            parameters: Some(json!({"type": "object"})),
            strict: None,
        },
    };

    let result: Result<FunctionDefinition, _> = openai.try_into_langchain();
    assert!(result.is_err(), "missing description must be rejected");
}

#[test]
fn function_definition_reverse_conversion_rejects_missing_parameters() {
    let openai = ChatCompletionTool {
        r#type: ChatCompletionToolType::Function,
        function: FunctionObject {
            name: "lookup_weather".to_owned(),
            description: Some("Looks up weather by city".to_owned()),
            parameters: None,
            strict: None,
        },
    };

    let result: Result<FunctionDefinition, _> = openai.try_into_langchain();
    assert!(result.is_err(), "missing parameters must be rejected");
}

#[test]
fn function_definition_reverse_conversion_rejects_strict_mode() {
    let openai = ChatCompletionTool {
        r#type: ChatCompletionToolType::Function,
        function: FunctionObject {
            name: "lookup_weather".to_owned(),
            description: Some("Looks up weather by city".to_owned()),
            parameters: Some(json!({"type": "object"})),
            strict: Some(true),
        },
    };

    let result: Result<FunctionDefinition, _> = openai.try_into_langchain();
    assert!(
        result.is_err(),
        "strict mode must not be silently discarded"
    );
}
