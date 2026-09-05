//! Builder for conversational retrieval-augmented question answering.
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    chain::{
        Chain, ChainError, CondenseQuestionGeneratorChain, DEFAULT_OUTPUT_KEY, StuffDocumentBuilder,
    },
    language_models::llm::{IntoArcLLM, LLM},
    memory::SimpleMemory,
    prompt::FormatPrompter,
    schemas::{BaseMemory, Retriever},
};

use super::ConversationalRetrieverChain;

const CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_INPUT_KEY: &str = "question";

/// Configures retrieval, question rewriting, document combination, and memory.
///
/// Supplying an LLM creates both internal chains during [`build`](Self::build). To use custom
/// chains, omit the LLM and configure both chain fields explicitly.
pub struct ConversationalRetrieverChainBuilder {
    llm: Option<Arc<dyn LLM>>,
    retriever: Option<Box<dyn Retriever>>,
    memory: Option<Arc<Mutex<dyn BaseMemory>>>,
    combine_documents_chain: Option<Box<dyn Chain>>,
    condense_question_chain: Option<Box<dyn Chain>>,
    prompt: Option<Box<dyn FormatPrompter>>,
    rephrase_question: bool,
    return_source_documents: bool,
    input_key: String,
    output_key: String,
}
#[allow(clippy::new_without_default)] // Builder pattern
impl ConversationalRetrieverChainBuilder {
    /// Creates a builder with question rewriting and source-document output enabled.
    pub fn new() -> Self {
        ConversationalRetrieverChainBuilder {
            llm: None,
            retriever: None,
            memory: None,
            combine_documents_chain: None,
            condense_question_chain: None,
            prompt: None,
            rephrase_question: true,
            return_source_documents: true,
            input_key: CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_INPUT_KEY.to_string(),
            output_key: DEFAULT_OUTPUT_KEY.to_string(),
        }
    }

    /// Sets the retriever used to find documents for each question.
    pub fn retriever<R: Into<Box<dyn Retriever>>>(mut self, retriever: R) -> Self {
        self.retriever = Some(retriever.into());
        self
    }

    /// Sets the `context` and `question` prompt used by the generated document chain.
    pub fn prompt<P: Into<Box<dyn FormatPrompter>>>(mut self, prompt: P) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the input key containing the user's question.
    pub fn input_key<S: Into<String>>(mut self, input_key: S) -> Self {
        self.input_key = input_key.into();
        self
    }

    /// Sets the shared conversation memory.
    pub fn memory(mut self, memory: Arc<Mutex<dyn BaseMemory>>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Sets the model used to create the default condensing and document chains.
    pub fn llm<L: IntoArcLLM>(mut self, llm: L) -> Self {
        self.llm = Some(llm.into_arc_llm());
        self
    }

    /// Sets the chain that generates an answer from documents and a question.
    pub fn combine_documents_chain<C: Into<Box<dyn Chain>>>(
        mut self,
        combine_documents_chain: C,
    ) -> Self {
        self.combine_documents_chain = Some(combine_documents_chain.into());
        self
    }

    /// Sets the chain that rewrites a question using chat history.
    pub fn condense_question_chain<C: Into<Box<dyn Chain>>>(
        mut self,
        condense_question_chain: C,
    ) -> Self {
        self.condense_question_chain = Some(condense_question_chain.into());
        self
    }

    /// Controls whether follow-up questions are rewritten when history is available.
    pub fn rephrase_question(mut self, rephrase_question: bool) -> Self {
        self.rephrase_question = rephrase_question;
        self
    }

    /// Controls whether [`Chain::execute`] includes retrieved documents in its output map.
    pub fn return_source_documents(mut self, return_source_documents: bool) -> Self {
        self.return_source_documents = return_source_documents;
        self
    }

    // qual:allow(iosp) reason: "builder validation + construction"
    /// Builds the chain, requiring a retriever and both internal chains or an LLM.
    pub fn build(mut self) -> Result<ConversationalRetrieverChain, ChainError> {
        if let Some(llm) = self.llm {
            let combine_documents_chain = {
                let mut builder = StuffDocumentBuilder::new().llm(llm.clone());
                if let Some(prompt) = self.prompt {
                    builder = builder.prompt(prompt);
                }
                builder.build()?
            };
            let condense_question_chain = CondenseQuestionGeneratorChain::new(llm.clone());
            self.combine_documents_chain = Some(Box::new(combine_documents_chain));
            self.condense_question_chain = Some(Box::new(condense_question_chain));
        }

        let retriever = self
            .retriever
            .ok_or_else(|| ChainError::MissingObject("Retriever must be set".into()))?;

        let memory = self
            .memory
            .unwrap_or_else(|| Arc::new(Mutex::new(SimpleMemory::new())));

        let combine_documents_chain = self.combine_documents_chain.ok_or_else(|| {
            ChainError::MissingObject(
                "Combine documents chain must be set or llm must be set".into(),
            )
        })?;
        let condense_question_chain = self.condense_question_chain.ok_or_else(|| {
            ChainError::MissingObject(
                "Condense question chain must be set or llm must be set".into(),
            )
        })?;
        Ok(ConversationalRetrieverChain {
            retriever,
            memory,
            combine_documents_chain,
            condense_question_chain,
            rephrase_question: self.rephrase_question,
            return_source_documents: self.return_source_documents,
            input_key: self.input_key,
            output_key: self.output_key,
        })
    }
}
