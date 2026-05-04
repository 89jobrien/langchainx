use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::{
    language_models::{llm::IntoArcLLM, GenerateResult},
    prompt::PromptArgs,
    prompt_args,
    schemas::{messages::Message, Document, StreamData},
    template_jinja2,
};

use super::{
    options::ChainCallOptions, Chain, ChainError, LLMChain, LLMChainBuilder, StuffDocument,
};

const DEFAULTCONDENSEQUESTIONTEMPLATE: &str = r#"Given the following conversation and a follow up question, rephrase the follow up question to be a standalone question, in its original language.

Chat History:
{{chat_history}}
Follow Up Input: {{question}}
Standalone question:"#;

pub struct CondenseQuestionPromptBuilder {
    chat_history: String,
    question: String,
}
impl CondenseQuestionPromptBuilder {
    pub fn new() -> Self {
        Self {
            chat_history: "".to_string(),
            question: "".to_string(),
        }
    }

    pub fn question<S: Into<String>>(mut self, question: S) -> Self {
        self.question = question.into();
        self
    }

    pub fn chat_history(mut self, chat_history: &[Message]) -> Self {
        self.chat_history = Message::messages_to_string(chat_history);
        self
    }

    pub fn build(self) -> PromptArgs {
        prompt_args! {
            "chat_history" => self.chat_history,
            "question" => self.question
        }
    }
}

pub struct CondenseQuestionGeneratorChain {
    chain: LLMChain,
}

impl CondenseQuestionGeneratorChain {
    pub fn new<L: IntoArcLLM>(llm: L) -> Self {
        let condense_question_prompt_template =
            template_jinja2!(DEFAULTCONDENSEQUESTIONTEMPLATE, "chat_history", "question");

        let chain = LLMChainBuilder::new()
            .llm(llm)
            .prompt(condense_question_prompt_template)
            .build()
            .unwrap(); //Its safe to unwrap here because we are sure that the prompt and the LLM are
                       //set.
        Self { chain }
    }

    pub fn prompt_builder(&self) -> CondenseQuestionPromptBuilder {
        CondenseQuestionPromptBuilder::new()
    }
}

#[async_trait]
impl Chain for CondenseQuestionGeneratorChain {
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        self.chain.call(input_variables).await
    }

    async fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>
    {
        self.chain.stream(input_variables).await
    }
}

const DEFAULT_STUFF_QA_TEMPLATE: &str = r#"Use the following pieces of context to answer the question at the end. If you don't know the answer, just say that you don't know, don't try to make up an answer.

{{context}}

Question:{{question}}
Helpful Answer:
"#;

pub struct StuffQAPromptBuilder<'a> {
    input_documents: Vec<&'a Document>,
    question: String,
}

impl<'a> StuffQAPromptBuilder<'a> {
    pub fn new() -> Self {
        Self {
            input_documents: vec![],
            question: "".to_string(),
        }
    }

    pub fn documents(mut self, documents: &'a [Document]) -> Self {
        self.input_documents = documents.iter().collect();
        self
    }

    pub fn question<S: Into<String>>(mut self, question: S) -> Self {
        self.question = question.into();
        self
    }

    pub fn build(self) -> PromptArgs {
        prompt_args! {
            "input_documents" => self.input_documents,
            "question" => self.question
        }
    }
}

pub(crate) fn load_stuff_qa<L: IntoArcLLM>(
    llm: L,
    options: Option<ChainCallOptions>,
) -> StuffDocument {
    let default_qa_prompt_template =
        template_jinja2!(DEFAULT_STUFF_QA_TEMPLATE, "context", "question");

    let llm_chain_builder = LLMChainBuilder::new()
        .prompt(default_qa_prompt_template)
        .options(options.unwrap_or_default())
        .llm(llm)
        .build()
        .unwrap();

    let llm_chain = llm_chain_builder;

    StuffDocument::new(llm_chain)
}

#[cfg(test)]
mod tests {
    use crate::{
        chain::{Chain, StuffDocument},
        schemas::Document,
        test_utils::FakeLLM,
    };

    #[tokio::test]
    async fn stuff_qa_returns_fake_response() {
        let llm = FakeLLM::new(vec!["Nvim, age 24".into()]);
        let chain = StuffDocument::load_stuff_qa(llm);
        let input = chain
            .qa_prompt_builder()
            .documents(&[
                Document::new("Q: favorite editor? A: Nvim"),
                Document::new("Q: age? A: 24"),
            ])
            .question("editor and age?")
            .build();

        let result = chain.invoke(input).await.expect("qa invoke failed");
        assert_eq!(result, "Nvim, age 24");
    }

    #[tokio::test]
    async fn stuff_qa_empty_documents_still_calls_llm() {
        let llm = FakeLLM::new(vec!["I don't know".into()]);
        let call_count = llm.call_count.clone();
        let chain = StuffDocument::load_stuff_qa(llm);
        let input = chain
            .qa_prompt_builder()
            .documents(&[])
            .question("anything?")
            .build();

        let result = chain.invoke(input).await.expect("qa invoke failed");
        assert_eq!(result, "I don't know");
        assert_eq!(call_count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
