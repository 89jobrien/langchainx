//! Builder for a stateful conversational chain.
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    chain::{
        ChainError, DEFAULT_OUTPUT_KEY, llm_chain::LLMChainBuilder, options::ChainCallOptions,
    },
    language_models::llm::{DynLLM, IntoArcLLM},
    memory::SimpleMemory,
    output_parsers::OutputParser,
    prompt::{FormatPrompter, HumanMessagePromptTemplate},
    schemas::memory::BaseMemory,
    template_fstring,
};

use super::{ConversationalChain, DEFAULT_INPUT_VARIABLE, prompt::DEFAULT_TEMPLATE};

/// Configures the model, prompt, memory, parser, and keys for a [`ConversationalChain`].
pub struct ConversationalChainBuilder {
    llm: Option<Arc<dyn DynLLM>>,
    options: Option<ChainCallOptions>,
    memory: Option<Arc<Mutex<dyn BaseMemory>>>,
    output_key: Option<String>,
    output_parser: Option<Box<dyn OutputParser>>,
    input_key: Option<String>,
    prompt: Option<Box<dyn FormatPrompter>>,
}

#[allow(clippy::new_without_default)] // Builder pattern; Default would be misleading
impl ConversationalChainBuilder {
    /// Creates an empty builder that uses default memory, prompt, and keys.
    pub fn new() -> Self {
        Self {
            llm: None,
            options: None,
            memory: None,
            output_key: None,
            output_parser: None,
            input_key: None,
            prompt: None,
        }
    }

    /// Sets the language model used for each conversation turn.
    pub fn llm<L: IntoArcLLM>(mut self, llm: L) -> Self {
        self.llm = Some(llm.into_arc_llm());
        self
    }

    /// Sets model call options.
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    /// Sets the prompt-argument key containing the user's message.
    pub fn input_key<S: Into<String>>(mut self, input_key: S) -> Self {
        self.input_key = Some(input_key.into());
        self
    }

    /// Sets the parser applied to model generations.
    pub fn output_parser<P: Into<Box<dyn OutputParser>>>(mut self, output_parser: P) -> Self {
        self.output_parser = Some(output_parser.into());
        self
    }

    /// Sets the shared conversation memory.
    pub fn memory(mut self, memory: Arc<Mutex<dyn BaseMemory>>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Sets the key used for generated text in structured output.
    pub fn output_key<S: Into<String>>(mut self, output_key: S) -> Self {
        self.output_key = Some(output_key.into());
        self
    }

    /// Sets a custom prompt, which must accept `history` and the configured input key.
    pub fn prompt<P: Into<Box<dyn FormatPrompter>>>(mut self, prompt: P) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    // qual:allow(iosp) reason: "builder validation + construction"
    /// Builds the chain, requiring a language model.
    pub fn build(self) -> Result<ConversationalChain, ChainError> {
        let llm = self
            .llm
            .ok_or_else(|| ChainError::MissingObject("LLM must be set".into()))?;
        let prompt = match self.prompt {
            Some(prompt) => prompt,
            None => Box::new(HumanMessagePromptTemplate::new(template_fstring!(
                DEFAULT_TEMPLATE,
                "history",
                "input"
            ))),
        };
        let llm_chain = {
            let mut builder = LLMChainBuilder::new()
                .prompt(prompt)
                .llm(llm)
                .output_key(self.output_key.unwrap_or_else(|| DEFAULT_OUTPUT_KEY.into()));

            if let Some(options) = self.options {
                builder = builder.options(options);
            }

            if let Some(output_parser) = self.output_parser {
                builder = builder.output_parser(output_parser);
            }

            builder.build()?
        };

        let memory = self
            .memory
            .unwrap_or_else(|| Arc::new(Mutex::new(SimpleMemory::new())));

        Ok(ConversationalChain {
            llm: llm_chain,
            memory,
            input_key: self
                .input_key
                .unwrap_or_else(|| DEFAULT_INPUT_VARIABLE.to_string()),
        })
    }
}
