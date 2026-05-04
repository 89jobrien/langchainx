use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::Stream;
use futures_util::TryStreamExt;

use crate::{
    language_models::{
        llm::{IntoArcLLM, LLM},
        GenerateResult,
    },
    output_parsers::{OutputParser, SimpleParser},
    prompt::{FormatPrompter, PromptArgs},
    schemas::StreamData,
};

use super::{chain_trait::Chain, options::ChainCallOptions, ChainError};

pub struct LLMChainBuilder {
    prompt: Option<Box<dyn FormatPrompter>>,
    llm: Option<Arc<dyn LLM>>,
    output_key: Option<String>,
    options: Option<ChainCallOptions>,
    output_parser: Option<Box<dyn OutputParser>>,
}

impl LLMChainBuilder {
    pub fn new() -> Self {
        Self {
            prompt: None,
            llm: None,
            options: None,
            output_key: None,
            output_parser: None,
        }
    }
    pub fn options(mut self, options: ChainCallOptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn prompt<P: Into<Box<dyn FormatPrompter>>>(mut self, prompt: P) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    pub fn llm<L: IntoArcLLM>(mut self, llm: L) -> Self {
        self.llm = Some(llm.into_arc_llm());
        self
    }

    pub fn output_key<S: Into<String>>(mut self, output_key: S) -> Self {
        self.output_key = Some(output_key.into());
        self
    }

    pub fn output_parser<P: Into<Box<dyn OutputParser>>>(mut self, output_parser: P) -> Self {
        self.output_parser = Some(output_parser.into());
        self
    }

    pub fn build(self) -> Result<LLMChain, ChainError> {
        let prompt = self
            .prompt
            .ok_or_else(|| ChainError::MissingObject("Prompt must be set".into()))?;

        let mut llm = self
            .llm
            .ok_or_else(|| ChainError::MissingObject("LLM must be set".into()))?;

        if let Some(options) = self.options {
            let llm_options = ChainCallOptions::to_llm_options(options);
            if let Some(llm_mut) = Arc::get_mut(&mut llm) {
                llm_mut.add_options(llm_options);
            } else {
                log::warn!(
                    "LLMChain: Arc<dyn LLM> is shared; chain-level options were not applied. \
                     Pass options directly to the LLM before wrapping in Arc."
                );
            }
        }

        let chain = LLMChain {
            prompt,
            llm,
            output_key: self.output_key.unwrap_or("output".to_string()),
            output_parser: self
                .output_parser
                .unwrap_or_else(|| Box::new(SimpleParser::default())),
        };

        Ok(chain)
    }
}

pub struct LLMChain {
    prompt: Box<dyn FormatPrompter>,
    llm: Arc<dyn LLM>,
    output_key: String,
    output_parser: Box<dyn OutputParser>,
}

#[async_trait]
impl Chain for LLMChain {
    fn required_keys(&self) -> Vec<String> {
        self.prompt.get_input_variables()
    }

    fn get_input_keys(&self) -> Vec<String> {
        self.prompt.get_input_variables()
    }

    fn get_output_keys(&self) -> Vec<String> {
        vec![self.output_key.clone()]
    }

    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        self.validate_input(&input_variables)?;
        let prompt = self.prompt.format_prompt(input_variables.clone())?;
        log::debug!("Prompt: {:?}", prompt);
        let mut output = self.llm.generate(&prompt.to_chat_messages()).await?;
        output.generation = self.output_parser.parse(&output.generation).await?;

        Ok(output)
    }

    async fn invoke(&self, input_variables: PromptArgs) -> Result<String, ChainError> {
        self.validate_input(&input_variables)?;
        let prompt = self.prompt.format_prompt(input_variables.clone())?;
        log::debug!("Prompt: {:?}", prompt);
        let output = self
            .llm
            .generate(&prompt.to_chat_messages())
            .await?
            .generation;
        Ok(output)
    }

    async fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>
    {
        let prompt = self.prompt.format_prompt(input_variables.clone())?;
        log::debug!("Prompt: {:?}", prompt);
        let llm_stream = self.llm.stream(&prompt.to_chat_messages()).await?;

        // Map the errors from LLMError to ChainError
        let mapped_stream = llm_stream.map_err(ChainError::from);

        Ok(Box::pin(mapped_stream))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        chain::Chain,
        message_formatter,
        prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
        prompt_args, template_fstring,
        test_utils::FakeLLM,
    };

    use super::*;

    fn make_chain(responses: Vec<String>) -> LLMChain {
        let prompt = message_formatter![MessageOrTemplate::Template(
            HumanMessagePromptTemplate::new(template_fstring!("Hello {input}", "input")).into()
        )];
        LLMChainBuilder::new()
            .prompt(prompt)
            .llm(FakeLLM::new(responses))
            .build()
            .expect("failed to build LLMChain")
    }

    #[tokio::test]
    async fn invoke_returns_fake_response() {
        let chain = make_chain(vec!["hello world".into()]);
        let result = chain.invoke(prompt_args! { "input" => "hi" }).await;
        assert_eq!(result.unwrap(), "hello world");
    }

    #[tokio::test]
    async fn call_returns_generate_result() {
        let chain = make_chain(vec!["the answer".into()]);
        let result = chain.call(prompt_args! { "input" => "question" }).await;
        assert_eq!(result.unwrap().generation, "the answer");
    }

    #[tokio::test]
    async fn missing_input_variable_returns_error() {
        let chain = make_chain(vec!["x".into()]);
        // "input" key is required by the prompt but not provided
        let result = chain.invoke(prompt_args! { "wrong_key" => "val" }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn exhausted_fake_llm_returns_empty_string() {
        let chain = make_chain(vec![]);
        let result = chain.invoke(prompt_args! { "input" => "hi" }).await;
        assert_eq!(result.unwrap(), "");
    }
}
