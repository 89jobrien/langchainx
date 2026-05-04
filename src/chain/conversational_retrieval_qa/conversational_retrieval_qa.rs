use futures::Stream;
use futures_util::{StreamExt, pin_mut};
use std::{collections::HashMap, pin::Pin, sync::Arc};

use async_stream::stream;
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::Mutex;

use crate::{
    chain::{
        Chain, ChainError, CondenseQuestionPromptBuilder, DEFAULT_RESULT_KEY, StuffQAPromptBuilder,
    },
    language_models::{GenerateResult, TokenUsage},
    prompt::PromptArgs,
    schemas::{BaseMemory, Message, Retriever, StreamData},
};
// _conversationalRetrievalQADefaultInputKey             = "question"
// _conversationalRetrievalQADefaultSourceDocumentKey    = "source_documents"
// 	_conversationalRetrievalQADefaultGeneratedQuestionKey = "generated_question"
// )

const CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_SOURCE_DOCUMENT_KEY: &str = "source_documents";
const CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_GENERATED_QUESTION_KEY: &str = "generated_question";

pub struct ConversationalRetrieverChain {
    pub(crate) retriever: Box<dyn Retriever>,
    pub memory: Arc<Mutex<dyn BaseMemory>>,
    pub(crate) combine_documents_chain: Box<dyn Chain>,
    pub(crate) condense_question_chain: Box<dyn Chain>,
    pub(crate) rephrase_question: bool,
    pub(crate) return_source_documents: bool,
    pub(crate) input_key: String,  //Default is `question`
    pub(crate) output_key: String, //default is output
}
impl ConversationalRetrieverChain {
    async fn get_question(
        &self,
        history: &[Message],
        input: &str,
    ) -> Result<(String, Option<TokenUsage>), ChainError> {
        if history.is_empty() {
            return Ok((input.to_string(), None));
        }
        let mut token_usage: Option<TokenUsage> = None;
        let question = match self.rephrase_question {
            true => {
                let result = self
                    .condense_question_chain
                    .call(
                        CondenseQuestionPromptBuilder::new()
                            .question(input)
                            .chat_history(history)
                            .build(),
                    )
                    .await?;
                if let Some(tokens) = result.tokens {
                    token_usage = Some(tokens);
                };
                result.generation
            }
            false => input.to_string(),
        };

        Ok((question, token_usage))
    }
}

#[async_trait]
impl Chain for ConversationalRetrieverChain {
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        let output = self.execute(input_variables).await?;
        let result: GenerateResult = serde_json::from_value(output[DEFAULT_RESULT_KEY].clone())?;
        Ok(result)
    }

    async fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> Result<HashMap<String, Value>, ChainError> {
        let mut token_usage: Option<TokenUsage> = None;
        let input_variable = &input_variables
            .get(&self.input_key)
            .ok_or(ChainError::MissingInputVariable(self.input_key.clone()))?;

        let human_message = Message::new_human_message(input_variable);
        let history = {
            let memory = self.memory.lock().await;
            memory.messages()
        };

        let (question, token) = self.get_question(&history, &human_message.content).await?;
        if let Some(token) = token {
            token_usage = Some(token);
        }

        let documents = self
            .retriever
            .get_relevant_documents(&question)
            .await
            .map_err(|e| ChainError::RetrieverError(e.to_string()))?;

        let mut output = self
            .combine_documents_chain
            .call(
                StuffQAPromptBuilder::new()
                    .documents(&documents)
                    .question(question.clone())
                    .build(),
            )
            .await?;

        match &output.tokens {
            Some(tokens) => {
                if let Some(mut token_usage) = token_usage {
                    token_usage.add(tokens);
                    output.tokens = Some(token_usage)
                }
            }
            None => {}
        }

        {
            let mut memory = self.memory.lock().await;
            memory.add_message(human_message);
            memory.add_message(Message::new_ai_message(&output.generation));
        }

        let mut result = HashMap::new();
        result.insert(self.output_key.clone(), json!(output.generation));

        result.insert(DEFAULT_RESULT_KEY.to_string(), json!(output));

        if self.return_source_documents {
            result.insert(
                CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_SOURCE_DOCUMENT_KEY.to_string(),
                json!(documents),
            );
        }

        if self.rephrase_question {
            result.insert(
                CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_GENERATED_QUESTION_KEY.to_string(),
                json!(question),
            );
        }

        Ok(result)
    }

    async fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>
    {
        let input_variable = &input_variables
            .get(&self.input_key)
            .ok_or(ChainError::MissingInputVariable(self.input_key.clone()))?;

        let human_message = Message::new_human_message(input_variable);
        let history = {
            let memory = self.memory.lock().await;
            memory.messages()
        };

        let (question, _) = self.get_question(&history, &human_message.content).await?;

        let documents = self
            .retriever
            .get_relevant_documents(&question)
            .await
            .map_err(|e| ChainError::RetrieverError(e.to_string()))?;

        let stream = self
            .combine_documents_chain
            .stream(
                StuffQAPromptBuilder::new()
                    .documents(&documents)
                    .question(question.clone())
                    .build(),
            )
            .await?;

        let memory = self.memory.clone();
        let complete_ai_message = Arc::new(Mutex::new(String::new()));
        let complete_ai_message_clone = complete_ai_message.clone();
        let output_stream = stream! {
            pin_mut!(stream);
            while let Some(result) = stream.next().await {
                match result {
                    Ok(data) => {
                        let mut complete_ai_message_clone =
                            complete_ai_message_clone.lock().await;
                        complete_ai_message_clone.push_str(&data.content);

                        yield Ok(data);
                    },
                    Err(e) => {
                        yield Err(e);
                    }
                }
            }

            let mut memory = memory.lock().await;
            memory.add_message(human_message);
            memory.add_message(Message::new_ai_message(&complete_ai_message.lock().await));
        };

        Ok(Box::pin(output_stream))
    }

    fn get_input_keys(&self) -> Vec<String> {
        vec![self.input_key.clone()]
    }

    fn get_output_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        if self.return_source_documents {
            keys.push(CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_SOURCE_DOCUMENT_KEY.to_string());
        }

        if self.rephrase_question {
            keys.push(CONVERSATIONAL_RETRIEVAL_QA_DEFAULT_GENERATED_QUESTION_KEY.to_string());
        }

        keys.push(self.output_key.clone());
        keys.push(DEFAULT_RESULT_KEY.to_string());

        keys
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use crate::{
        chain::ConversationalRetrieverChainBuilder, memory::SimpleMemory, prompt_args,
        schemas::Document, test_utils::FakeLLM,
    };

    use super::*;

    struct RetrieverTest {}

    #[async_trait]
    impl Retriever for RetrieverTest {
        async fn get_relevant_documents(
            &self,
            _question: &str,
        ) -> Result<Vec<Document>, Box<dyn Error>> {
            Ok(vec![
                Document::new("Q: favorite editor? A: Nvim"),
                Document::new("Q: age? A: 24"),
            ])
        }
    }

    #[tokio::test]
    async fn retriever_chain_returns_fake_response() {
        let llm = FakeLLM::new(vec!["Nvim".into()]);
        let chain = ConversationalRetrieverChainBuilder::new()
            .llm(llm)
            .retriever(RetrieverTest {})
            .memory(SimpleMemory::new().into())
            .build()
            .expect("failed to build ConversationalRetrieverChain");

        let result = chain
            .invoke(prompt_args! { "question" => "favorite editor?" })
            .await
            .expect("invoke failed");

        assert_eq!(result, "Nvim");
    }

    #[tokio::test]
    async fn retriever_chain_accumulates_memory() {
        let llm = FakeLLM::new(vec!["first answer".into(), "second answer".into()]);
        let chain = ConversationalRetrieverChainBuilder::new()
            .llm(llm)
            .retriever(RetrieverTest {})
            .memory(SimpleMemory::new().into())
            .build()
            .expect("failed to build ConversationalRetrieverChain");

        chain
            .invoke(prompt_args! { "question" => "first?" })
            .await
            .expect("first invoke failed");

        chain
            .invoke(prompt_args! { "question" => "second?" })
            .await
            .expect("second invoke failed");

        let memory = chain.memory.lock().await;
        assert_eq!(memory.messages().len(), 4); // 2 human + 2 ai
    }
}
