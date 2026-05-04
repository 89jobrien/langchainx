use std::{pin::Pin, sync::Arc};

use async_stream::stream;
use async_trait::async_trait;
use futures::Stream;
use futures_util::{StreamExt, pin_mut};
use tokio::sync::Mutex;

use crate::{
    language_models::GenerateResult,
    prompt::PromptArgs,
    prompt_args,
    schemas::{StreamData, memory::BaseMemory, messages::Message},
};

const DEFAULT_INPUT_VARIABLE: &str = "input";

use super::{ChainError, chain_trait::Chain, llm_chain::LLMChain};

pub mod builder;
mod prompt;

///This is only usefull when you dont modify the original prompt
pub struct ConversationalChainPromptBuilder {
    input: String,
}

impl ConversationalChainPromptBuilder {
    pub fn new() -> Self {
        Self {
            input: "".to_string(),
        }
    }

    pub fn input<S: Into<String>>(mut self, input: S) -> Self {
        self.input = input.into();
        self
    }

    pub fn build(self) -> PromptArgs {
        prompt_args! {
            DEFAULT_INPUT_VARIABLE => self.input,
        }
    }
}

pub struct ConversationalChain {
    llm: LLMChain,
    input_key: String,
    pub memory: Arc<Mutex<dyn BaseMemory>>,
}

//Conversational Chain is a simple chain to interact with ai as a string of messages
impl ConversationalChain {
    pub fn prompt_builder(&self) -> ConversationalChainPromptBuilder {
        ConversationalChainPromptBuilder::new()
    }
}

#[async_trait]
impl Chain for ConversationalChain {
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        let input_variable = &input_variables
            .get(&self.input_key)
            .ok_or(ChainError::MissingInputVariable(self.input_key.clone()))?;
        let human_message = Message::new_human_message(input_variable);

        let history = {
            let memory = self.memory.lock().await;
            memory.to_string()
        };
        let mut input_variables = input_variables;
        input_variables.insert("history".to_string(), history.into());
        let result = self.llm.call(input_variables.clone()).await?;

        let mut memory = self.memory.lock().await;
        memory.add_message(human_message);
        memory.add_message(Message::new_ai_message(&result.generation));
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
            memory.to_string()
        };

        let mut input_variables = input_variables;
        input_variables.insert("history".to_string(), history.into());

        let complete_ai_message = Arc::new(Mutex::new(String::new()));
        let complete_ai_message_clone = complete_ai_message.clone();

        let memory = self.memory.clone();

        let stream = self.llm.stream(input_variables).await?;
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
}

#[cfg(test)]
mod tests {
    use crate::{
        chain::{Chain, conversational::builder::ConversationalChainBuilder},
        prompt_args,
        test_utils::FakeLLM,
    };

    #[tokio::test]
    async fn conversational_chain_returns_fake_response() {
        let llm = FakeLLM::new(vec!["response one".into()]);
        let chain = ConversationalChainBuilder::new()
            .llm(llm)
            .build()
            .expect("failed to build ConversationalChain");

        let result = chain
            .invoke(prompt_args! { "input" => "hello" })
            .await
            .expect("invoke failed");

        assert_eq!(result, "response one");
    }

    #[tokio::test]
    async fn conversational_chain_accumulates_history() {
        let llm = FakeLLM::new(vec!["first reply".into(), "second reply".into()]);
        let chain = ConversationalChainBuilder::new()
            .llm(llm)
            .build()
            .expect("failed to build ConversationalChain");

        let r1 = chain
            .invoke(prompt_args! { "input" => "turn one" })
            .await
            .expect("first invoke failed");
        assert_eq!(r1, "first reply");

        let r2 = chain
            .invoke(prompt_args! { "input" => "turn two" })
            .await
            .expect("second invoke failed");
        assert_eq!(r2, "second reply");

        // Memory should contain both turns (2 human + 2 ai = 4 messages)
        let memory = chain.memory.lock().await;
        assert_eq!(memory.messages().len(), 4);
    }

    #[tokio::test]
    async fn conversational_chain_missing_input_key_returns_error() {
        let llm = FakeLLM::new(vec!["x".into()]);
        let chain = ConversationalChainBuilder::new()
            .llm(llm)
            .build()
            .expect("failed to build ConversationalChain");

        let result = chain.invoke(prompt_args! { "wrong_key" => "val" }).await;
        assert!(result.is_err());
    }
}
