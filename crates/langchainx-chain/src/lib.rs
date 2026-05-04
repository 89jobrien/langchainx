#![allow(
    clippy::module_inception,
    clippy::new_without_default,
    clippy::single_match,
    dead_code
)]
// Re-export upstream crates so `crate::language_models`, `crate::schemas`,
// `crate::prompt`, `crate::output_parsers` all resolve within this crate.
// Use langchainx_llm::language_models (superset: includes LLM, LLMError, CallOptions + core types).
pub use langchainx_llm::language_models;
pub use langchainx_core::schemas;
pub use langchainx_output_parsers as output_parsers;
pub use langchainx_prompt::prompt;
pub use langchainx_memory as memory;

// Macro re-exports from langchainx-prompt
pub use langchainx_prompt::{
    fmt_message, fmt_placeholder, fmt_template, message_formatter, prompt_args,
    template_fstring, template_jinja2,
};

// Compatibility module: `crate::chain::X` paths used within chain files
pub mod chain;

pub mod chain_trait;
pub use chain_trait::*;

pub mod conversational;
pub use conversational::*;

pub mod llm_chain;
pub use llm_chain::*;

mod sequential;
pub use sequential::*;

#[cfg(feature = "postgres")]
pub mod sql_datbase;
#[cfg(feature = "postgres")]
pub use sql_datbase::*;

mod stuff_documents;
pub use stuff_documents::*;

mod question_answering;
pub use question_answering::*;

mod conversational_retrieval_qa;
pub use conversational_retrieval_qa::*;

mod error;
pub use error::*;

pub mod options;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;
