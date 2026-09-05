//! Chain that joins document contents and passes them to an LLM chain.
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;
use serde_json::Value;

use crate::{
    chain::{
        Chain, ChainError, LLMChain, StuffQAPromptBuilder, load_stuff_qa, options::ChainCallOptions,
    },
    language_models::{
        GenerateResult,
        llm::{IntoArcLLM, LLM},
    },
    prompt::PromptArgs,
    schemas::{Document, StreamData},
};

const COMBINE_DOCUMENTS_DEFAULT_INPUT_KEY: &str = "input_documents";
const _COMBINE_DOCUMENTS_DEFAULT_OUTPUT_KEY: &str = "text";
const COMBINE_DOCUMENTS_DEFAULT_DOCUMENT_VARIABLE_NAME: &str = "context";
const STUFF_DOCUMENTS_DEFAULT_SEPARATOR: &str = "\n\n";

/// Combines input documents into a single prompt variable before generation.
pub struct StuffDocument {
    llm_chain: LLMChain,
    input_key: String,
    document_variable_name: String,
    separator: String,
}

impl StuffDocument {
    /// Wraps an LLM chain using the default document input key, context key, and separator.
    pub fn new(llm_chain: LLMChain) -> Self {
        Self {
            llm_chain,
            input_key: COMBINE_DOCUMENTS_DEFAULT_INPUT_KEY.to_string(),
            document_variable_name: COMBINE_DOCUMENTS_DEFAULT_DOCUMENT_VARIABLE_NAME.to_string(),
            separator: STUFF_DOCUMENTS_DEFAULT_SEPARATOR.to_string(),
        }
    }

    fn join_documents(&self, docs: Vec<Document>) -> String {
        docs.iter()
            .map(|doc| doc.page_content.clone())
            .collect::<Vec<_>>()
            .join(&self.separator)
    }

    /// Creates an input builder for the default question-answering prompt.
    pub fn qa_prompt_builder<'a>(&self) -> StuffQAPromptBuilder<'a> {
        StuffQAPromptBuilder::new()
    }

    /// Creates a document chain with the default question-answering prompt.
    ///
    /// # Example
    /// ```rust,ignore
    ///
    /// let llm = OpenAI::default();
    /// let chain = StuffDocument::load_stuff_qa(llm);
    ///
    /// let input = chain
    /// .qa_prompt_builder()
    /// .documents(&[
    /// Document::new(format!(
    /// "\nQuestion: {}\nAnswer: {}\n",
    /// "Which is the favorite text editor of luis", "Nvim"
    /// )),
    /// Document::new(format!(
    /// "\nQuestion: {}\nAnswer: {}\n",
    /// "How old is Luis", "24"
    /// )),
    /// ])
    /// .question("How old is luis and whats his favorite text editor")
    /// .build();
    ///
    /// let ouput = chain.invoke(input).await.unwrap();
    ///
    /// println!("{}", ouput);
    /// ```
    ///
    pub fn load_stuff_qa<L: IntoArcLLM>(llm: L) -> Self {
        load_stuff_qa(llm, None)
    }

    /// Creates a document chain with the default question-answering prompt and model options.
    ///
    /// Input construction is identical to [`load_stuff_qa`](Self::load_stuff_qa).
    pub fn load_stuff_qa_with_options<L: LLM + 'static>(llm: L, opt: ChainCallOptions) -> Self {
        load_stuff_qa(llm, Some(opt))
    }
}

#[async_trait]
impl Chain for StuffDocument {
    fn required_keys(&self) -> Vec<String> {
        vec![self.input_key.clone()]
    }

    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        self.validate_input(&input_variables)?;
        let docs = &input_variables[&self.input_key];

        let documents: Vec<Document> = serde_json::from_value(docs.clone()).map_err(|e| {
            ChainError::IncorrectInputVariable {
                source: e,
                expected_type: "Vec<Document>".to_string(),
            }
        })?;

        let mut input_values = input_variables.clone();
        input_values.insert(
            self.document_variable_name.clone(),
            Value::String(self.join_documents(documents)),
        );

        self.llm_chain.call(input_values).await
    }

    async fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>
    {
        self.validate_input(&input_variables)?;
        let docs = &input_variables[&self.input_key];

        let documents: Vec<Document> = serde_json::from_value(docs.clone()).map_err(|e| {
            ChainError::IncorrectInputVariable {
                source: e,
                expected_type: "Vec<Document>".to_string(),
            }
        })?;

        let mut input_values = input_variables.clone();
        input_values.insert(
            self.document_variable_name.clone(),
            Value::String(self.join_documents(documents)),
        );
        self.llm_chain.stream(input_values).await
    }

    fn get_input_keys(&self) -> Vec<String> {
        vec![self.input_key.clone()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chain::{Chain, ChainError, LLMChainBuilder},
        message_formatter,
        prompt::{HumanMessagePromptTemplate, MessageOrTemplate},
        prompt_args,
        schemas::Document,
        template_fstring,
        test_utils::FakeLLM,
    };

    fn make_stuff_chain(responses: Vec<String>) -> StuffDocument {
        // Prompt expects {context} — matches COMBINE_DOCUMENTS_DEFAULT_DOCUMENT_VARIABLE_NAME
        let prompt = message_formatter![MessageOrTemplate::Template(
            HumanMessagePromptTemplate::new(template_fstring!("{context}", "context")).into()
        )];
        let llm_chain = LLMChainBuilder::new()
            .prompt(prompt)
            .llm(FakeLLM::new(responses))
            .build()
            .expect("failed to build LLMChain");
        StuffDocument::new(llm_chain)
    }

    #[tokio::test]
    async fn documents_are_joined_and_llm_response_returned() {
        let chain = make_stuff_chain(vec!["combined answer".into()]);
        let docs = vec![Document::new("first doc"), Document::new("second doc")];
        let input = prompt_args! {
            "input_documents" => docs
        };
        let result = chain.invoke(input).await.expect("invoke failed");
        assert_eq!(result, "combined answer");
    }

    #[tokio::test]
    async fn empty_document_list_still_calls_llm() {
        let llm = FakeLLM::new(vec!["empty response".into()]);
        let call_count = llm.call_count.clone();
        // Recreate with a tracked FakeLLM (make_stuff_chain doesn't expose call_count).
        let prompt = message_formatter![MessageOrTemplate::Template(
            HumanMessagePromptTemplate::new(template_fstring!("{context}", "context")).into()
        )];
        let llm_chain = LLMChainBuilder::new()
            .prompt(prompt)
            .llm(llm)
            .build()
            .unwrap();
        let chain = StuffDocument::new(llm_chain);
        let input = prompt_args! { "input_documents" => Vec::<Document>::new() };
        let result = chain
            .invoke(input)
            .await
            .expect("invoke with empty docs failed");
        assert_eq!(result, "empty response");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn missing_input_documents_key_returns_error() {
        let chain = make_stuff_chain(vec!["x".into()]);
        // Provide wrong key — "input_documents" is missing
        let input = prompt_args! { "wrong_key" => "value" };
        let result = chain.call(input).await;
        assert!(
            matches!(
                result,
                Err(ChainError::MissingInputVariable { ref key, .. })
                    if key == "input_documents"
            ),
            "expected MissingInputVariable error, got: {:?}",
            result
        );
    }
}
