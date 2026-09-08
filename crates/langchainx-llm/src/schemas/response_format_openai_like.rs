//! Provider-neutral response formats convertible to OpenAI request types.
use crate::schemas::convert::{LangchainFromOpenAI, OpenAIFromLangchain};

#[derive(Clone, Debug, PartialEq)]
/// The representation a model should use for its response.
pub enum ResponseFormat {
    /// Unstructured text.
    Text,
    /// A valid JSON object without a supplied schema.
    JsonObject,
    /// JSON constrained by a named schema.
    JsonSchema {
        /// Human-readable purpose of the schema.
        description: Option<String>,
        /// Name used to identify the schema.
        name: String,
        /// JSON Schema definition for the response.
        schema: Option<serde_json::Value>,
        /// Whether the provider should enforce strict schema conformance.
        strict: Option<bool>,
    },
}

impl OpenAIFromLangchain<ResponseFormat> for async_openai::types::ResponseFormat {
    fn from_langchain(langchain: ResponseFormat) -> Self {
        match langchain {
            ResponseFormat::Text => async_openai::types::ResponseFormat::Text,
            ResponseFormat::JsonObject => async_openai::types::ResponseFormat::JsonObject,
            ResponseFormat::JsonSchema {
                name,
                description,
                schema,
                strict,
            } => async_openai::types::ResponseFormat::JsonSchema {
                json_schema: async_openai::types::ResponseFormatJsonSchema {
                    name,
                    description,
                    schema,
                    strict,
                },
            },
        }
    }
}

impl LangchainFromOpenAI<async_openai::types::ResponseFormat> for ResponseFormat {
    fn from_openai(openai: async_openai::types::ResponseFormat) -> Self {
        match openai {
            async_openai::types::ResponseFormat::Text => Self::Text,
            async_openai::types::ResponseFormat::JsonObject => Self::JsonObject,
            async_openai::types::ResponseFormat::JsonSchema { json_schema } => Self::JsonSchema {
                description: json_schema.description,
                name: json_schema.name,
                schema: json_schema.schema,
                strict: json_schema.strict,
            },
        }
    }
}
