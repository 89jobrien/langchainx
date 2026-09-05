//! Declarative helpers for defining tools and constructing prompts, models, and chains.
#![deny(missing_docs)]
// Re-exports used by macros. Not part of the public API.
#[doc(hidden)]
pub use async_trait::async_trait;
#[doc(hidden)]
pub use langchainx_core::{tools::Tool, tools::ToolError};
#[doc(hidden)]
pub use serde_json;

// Re-exports for prompt! macro
#[doc(hidden)]
pub use langchainx_prompt::prompt::HumanMessagePromptTemplate;
#[doc(hidden)]
pub use langchainx_prompt::template_fstring;

// Re-exports for chain! macro
#[doc(hidden)]
pub use langchainx_chain::LLMChainBuilder;

// ---------------------------------------------------------------------------
// tool!
// ---------------------------------------------------------------------------

/// Define a `Tool` implementation with minimal boilerplate.
///
/// # Simple tool
///
/// ```ignore
/// tool!(MyTool, "Does something useful", |input| {
///     Ok(format!("got: {input}"))
/// });
/// ```
///
/// # With custom parameters schema
///
/// ```ignore
/// tool!(MyTool, "Does something", parameters = json!({...}), |input| {
///     Ok(format!("searched: {}", input["query"]))
/// });
/// ```
#[macro_export]
macro_rules! tool {
    ($name:ident, $desc:expr, |$input:ident| $body:expr) => {
        pub struct $name;

        #[$crate::async_trait]
        impl $crate::Tool for $name {
            fn name(&self) -> ::std::string::String {
                stringify!($name).to_string()
            }

            fn description(&self) -> ::std::string::String {
                ($desc).to_string()
            }

            async fn run(
                &self,
                $input: $crate::serde_json::Value,
            ) -> ::std::result::Result<::std::string::String, $crate::ToolError> {
                $body
            }
        }
    };

    ($name:ident, $desc:expr, parameters = $params:expr, |$input:ident| $body:expr) => {
        pub struct $name;

        #[$crate::async_trait]
        impl $crate::Tool for $name {
            fn name(&self) -> ::std::string::String {
                stringify!($name).to_string()
            }

            fn description(&self) -> ::std::string::String {
                ($desc).to_string()
            }

            fn parameters(&self) -> $crate::serde_json::Value {
                $params
            }

            async fn run(
                &self,
                $input: $crate::serde_json::Value,
            ) -> ::std::result::Result<::std::string::String, $crate::ToolError> {
                $body
            }
        }
    };
}

// ---------------------------------------------------------------------------
// llm!
// ---------------------------------------------------------------------------

/// Construct an LLM backend with builder-method sugar.
///
/// ```ignore
/// let llm = llm!(OpenAI);                           // OpenAI::default()
/// let llm = llm!(OpenAI, model = "gpt-4o");         // .with_model(...)
/// let llm = llm!(Claude, model = "claude-sonnet-4-20250514", max_tokens = 2048);
/// ```
///
/// Each `key = val` pair calls the corresponding `.with_key(val)` method.
#[macro_export]
macro_rules! llm {
    // Zero options
    ($backend:ty) => {
        <$backend>::default()
    };

    // Recursive peeling: peel one key=val, recurse on the rest
    (@build $builder:expr, model = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_model($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, max_tokens = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_max_tokens($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, api_key = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_api_key($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, temperature = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_temperature($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, top_p = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_top_p($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, stop_words = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_stop_words($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, options = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_options($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, config = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_config($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, base_url = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_base_url($val) $(, $rest_key = $rest_val)*)
    };
    (@build $builder:expr, json_mode = $val:expr $(, $rest_key:ident = $rest_val:expr)*) => {
        $crate::llm!(@build $builder.with_json_mode($val) $(, $rest_key = $rest_val)*)
    };
    // Base case: no more options
    (@build $builder:expr $(,)?) => {
        $builder
    };

    // Entry point with options
    ($backend:ty, $($key:ident = $val:expr),+ $(,)?) => {
        $crate::llm!(@build <$backend>::default(), $($key = $val),+)
    };
}

// ---------------------------------------------------------------------------
// prompt!
// ---------------------------------------------------------------------------

/// Create a `HumanMessagePromptTemplate` from an fstring template.
///
/// ```ignore
/// let p = prompt!("Capital of {country}?", "country");
/// let p = prompt!("Hello {name}, welcome to {place}!", "name", "place");
/// ```
///
/// Equivalent to:
/// ```ignore
/// HumanMessagePromptTemplate::new(template_fstring!("...", "var1", "var2"))
/// ```
#[macro_export]
macro_rules! prompt {
    ($template:expr, $($var:expr),+ $(,)?) => {
        $crate::HumanMessagePromptTemplate::new(
            $crate::template_fstring!($template, $($var),+)
        )
    };
}

// ---------------------------------------------------------------------------
// chain!
// ---------------------------------------------------------------------------

/// Wire a prompt and LLM into an `LLMChain`.
///
/// ```ignore
/// let chain = chain!(prompt, llm);
/// let chain = chain!(prompt, llm, output_key = "result");
/// ```
///
/// Panics if the builder fails (which cannot happen when both prompt and
/// llm are provided).
#[macro_export]
macro_rules! chain {
    ($prompt:expr, $llm:expr) => {
        $crate::LLMChainBuilder::new()
            .prompt($prompt)
            .llm($llm)
            .build()
            .expect("chain! build failed -- prompt and llm are required")
    };

    ($prompt:expr, $llm:expr, output_key = $key:expr) => {
        $crate::LLMChainBuilder::new()
            .prompt($prompt)
            .llm($llm)
            .output_key($key)
            .build()
            .expect("chain! build failed -- prompt and llm are required")
    };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -- tool! tests --

    tool!(DateTool, "Gets the current date", |_input| {
        Ok("2026-05-31".to_string())
    });

    tool!(
        SearchTool,
        "Searches for a query",
        parameters = json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Search query" }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
        |input| {
            let q = input["query"].as_str().unwrap_or("none");
            Ok(format!("results for: {q}"))
        }
    );

    #[tokio::test]
    async fn tool_simple_name_and_description() {
        let tool = DateTool;
        assert_eq!(Tool::name(&tool), "DateTool");
        assert_eq!(Tool::description(&tool), "Gets the current date");
    }

    #[tokio::test]
    async fn tool_simple_run() {
        let tool = DateTool;
        let result = Tool::run(&tool, json!("")).await.unwrap();
        assert_eq!(result, "2026-05-31");
    }

    #[tokio::test]
    async fn tool_with_parameters_schema() {
        let tool = SearchTool;
        let params = Tool::parameters(&tool);
        assert_eq!(params["type"], "object");
        assert!(params["properties"]["query"].is_object());
    }

    #[tokio::test]
    async fn tool_with_parameters_run() {
        let tool = SearchTool;
        let result = Tool::run(&tool, json!({"query": "rust"})).await.unwrap();
        assert_eq!(result, "results for: rust");
    }

    #[tokio::test]
    async fn tool_is_arc_dyn_compatible() {
        let tool: std::sync::Arc<dyn Tool> = std::sync::Arc::new(DateTool);
        assert_eq!(tool.name(), "DateTool");
    }

    // -- prompt! tests --

    #[test]
    fn prompt_creates_human_message_template() {
        use langchainx_prompt::prompt::MessageFormatter;
        let p = prompt!("Hello {name}!", "name");
        let vars = p.input_variables();
        assert_eq!(vars, vec!["name"]);
    }

    #[test]
    fn prompt_multi_var() {
        use langchainx_prompt::prompt::FormatPrompter;
        let p = prompt!("{a} and {b}", "a", "b");
        let vars = p.get_input_variables();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&"a".to_string()));
        assert!(vars.contains(&"b".to_string()));
    }

    #[test]
    fn prompt_format_substitutes() {
        use langchainx_prompt::prompt::FormatPrompter;
        let p = prompt!("Capital of {country}?", "country");
        let pv = p
            .format_prompt(langchainx_prompt::prompt_args! { "country" => "France" })
            .unwrap();
        let msgs = pv.to_chat_messages();
        assert_eq!(msgs.len(), 1);
        assert_eq!(
            msgs[0].message_type,
            langchainx_core::schemas::MessageType::HumanMessage
        );
        assert_eq!(msgs[0].content, "Capital of France?");
    }

    // -- llm! tests --

    #[test]
    fn llm_macro_builds_default_openai() {
        use langchainx_llm::openai::{OpenAI, OpenAIConfig};
        let llm: OpenAI<OpenAIConfig> = llm!(OpenAI<OpenAIConfig>);
        assert!(std::mem::size_of_val(&llm) > 0);
    }

    #[test]
    fn llm_macro_with_model_sets_model_field() {
        use langchainx_llm::openai::{OpenAI, OpenAIConfig};
        let llm = llm!(OpenAI<OpenAIConfig>, model = "gpt-4o-mini");
        assert!(std::mem::size_of_val(&llm) > 0);
    }

    // -- chain! tests --

    // Shared FakeLLM for chain tests
    use langchainx_core::schemas::{Message, StreamData};
    use langchainx_llm::language_models::{GenerateResult, LLMError, llm::LLM};

    struct FakeLLM(String);
    #[async_trait::async_trait]
    impl LLM for FakeLLM {
        async fn generate(&self, _msgs: &[Message]) -> Result<GenerateResult, LLMError> {
            Ok(GenerateResult {
                generation: self.0.clone(),
                ..Default::default()
            })
        }
        async fn stream(
            &self,
            _msgs: &[Message],
        ) -> Result<
            std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamData, LLMError>> + Send>>,
            LLMError,
        > {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn chain_basic_construction() {
        use langchainx_chain::Chain;
        let p = prompt!("Capital of {country}?", "country");
        let chain = chain!(p, FakeLLM("Paris".into()));
        let result = chain
            .invoke(langchainx_prompt::prompt_args! { "country" => "France" })
            .await
            .unwrap();
        assert_eq!(result, "Paris");
    }

    #[tokio::test]
    async fn chain_with_output_key() {
        use langchainx_chain::Chain;
        let p = prompt!("{q}", "q");
        let chain = chain!(p, FakeLLM("42".into()), output_key = "answer");
        let keys = chain.get_output_keys();
        assert_eq!(keys, vec!["answer"]);
    }
}
