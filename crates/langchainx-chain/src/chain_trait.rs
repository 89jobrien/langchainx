use std::{collections::HashMap, future::Future, pin::Pin};

use futures::Stream;
use serde_json::{Value, json};

use crate::{language_models::GenerateResult, prompt::PromptArgs, schemas::StreamData};

use super::ChainError;

pub(crate) const DEFAULT_OUTPUT_KEY: &str = "output";
pub(crate) const DEFAULT_RESULT_KEY: &str = "generate_result";
pub type ChainStream = Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>;

pub trait Chain: Sync + Send {
    /// Call the `Chain` and receive as output the result of the generation process along with
    /// additional information like token consumption. The input is a set of variables passed
    /// as a `PromptArgs` hashmap.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use crate::my_crate::{Chain, ConversationalChainBuilder, OpenAI, OpenAIModel, SimpleMemory, PromptArgs, prompt_args};
    /// # async {
    /// let llm = OpenAI::default().with_model(OpenAIModel::Gpt35);
    /// let memory = SimpleMemory::new();
    ///
    /// let chain = ConversationalChainBuilder::new()
    ///     .llm(llm)
    ///     .memory(memory.into())
    ///     .build().expect("Error building ConversationalChain");
    ///
    /// let input_variables = prompt_args! {
    ///     "input" => "Im from Peru",
    /// };
    ///
    /// match chain.call(input_variables).await {
    ///     Ok(result) => {
    ///         println!("Result: {:?}", result);
    ///     },
    ///     Err(e) => panic!("Error calling Chain: {:?}", e),
    /// };
    /// # };
    /// ```
    fn call(
        &self,
        input_variables: PromptArgs,
    ) -> impl Future<Output = Result<GenerateResult, ChainError>> + Send;

    /// Invoke the `Chain` and receive just the generation result as a String.
    /// The input is a set of variables passed as a `PromptArgs` hashmap.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use crate::my_crate::{Chain, ConversationalChainBuilder, OpenAI, OpenAIModel, SimpleMemory, PromptArgs, prompt_args};
    /// # async {
    /// let llm = OpenAI::default().with_model(OpenAIModel::Gpt35);
    /// let memory = SimpleMemory::new();
    ///
    /// let chain = ConversationalChainBuilder::new()
    ///     .llm(llm)
    ///     .memory(memory.into())
    ///     .build().expect("Error building ConversationalChain");
    ///
    /// let input_variables = prompt_args! {
    ///     "input" => "Im from Peru",
    /// };
    ///
    /// match chain.invoke(input_variables).await {
    ///     Ok(result) => {
    ///         println!("Result: {:?}", result);
    ///     },
    ///     Err(e) => panic!("Error invoking Chain: {:?}", e),
    /// };
    /// # };
    /// ```
    fn invoke(
        &self,
        input_variables: PromptArgs,
    ) -> impl Future<Output = Result<String, ChainError>> + Send {
        async move {
            self.call(input_variables)
                .await
                .map(|result| result.generation)
        }
    }

    /// Execute the `Chain` and return the result of the generation process
    /// along with additional information like token consumption formatted as a `HashMap`.
    /// The input is a set of variables passed as a `PromptArgs` hashmap.
    /// The key for the generated output is specified by the `get_output_keys`
    /// method (default key is `output`).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// # use crate::my_crate::{Chain, ConversationalChainBuilder, OpenAI, OpenAIModel, SimpleMemory, PromptArgs, prompt_args};
    /// # async {
    /// let llm = OpenAI::default().with_model(OpenAIModel::Gpt35);
    /// let memory = SimpleMemory::new();
    ///
    /// let chain = ConversationalChainBuilder::new()
    ///     .llm(llm)
    ///     .memory(memory.into())
    ///     .output_key("name")
    ///     .build().expect("Error building ConversationalChain");
    ///
    /// let input_variables = prompt_args! {
    ///     "input" => "Im from Peru",
    /// };
    ///
    /// match chain.execute(input_variables).await {
    ///     Ok(result) => {
    ///         println!("Result: {:?}", result);
    ///     },
    ///     Err(e) => panic!("Error executing Chain: {:?}", e),
    /// };
    /// # };
    /// ```
    fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> impl Future<Output = Result<HashMap<String, Value>, ChainError>> + Send {
        async move {
            log::info!("Using default implementation");
            let result = self.call(input_variables.clone()).await?;
            let mut output = HashMap::new();
            let output_key = self
                .get_output_keys()
                .first()
                .unwrap_or(&DEFAULT_OUTPUT_KEY.to_string())
                .clone();
            output.insert(output_key, json!(result.generation));
            output.insert(DEFAULT_RESULT_KEY.to_string(), json!(result));
            Ok(output)
        }
    }
    /// Stream the `Chain` and get an asynchronous stream of chain generations.
    /// The input is a set of variables passed as a `PromptArgs` hashmap.
    /// If the chain have memroy, the tream method will not be able to automaticaly
    /// set the memroy, bocause it will not know if the how to extract the output message
    /// out of the stram
    /// # Example
    ///
    /// ```rust,ignore
    /// # use futures::StreamExt;
    /// # use crate::my_crate::{Chain, LLMChainBuilder, OpenAI, fmt_message, fmt_template,
    /// #                      HumanMessagePromptTemplate, prompt_args, Message, template_fstring};
    /// # async {
    /// let open_ai = OpenAI::default();
    ///
    ///let prompt = message_formatter![
    ///fmt_message!(Message::new_system_message(
    ///"You are world class technical documentation writer."
    ///)),
    ///fmt_template!(HumanMessagePromptTemplate::new(template_fstring!(
    ///      "{input}", "input"
    ///)))
    ///];
    ///
    /// let chain = LLMChainBuilder::new()
    ///     .prompt(prompt)
    ///     .llm(open_ai.clone())
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut stream = chain.stream(
    /// prompt_args! {
    /// "input" => "Who is the writer of 20,000 Leagues Under the Sea?"
    /// }).await.unwrap();
    ///
    /// while let Some(result) = stream.next().await {
    ///     match result {
    ///         Ok(value) => {
    ///                 println!("Content: {}", value.content);
    ///         },
    ///         Err(e) => panic!("Error invoking LLMChain: {:?}", e),
    ///     }
    /// };
    /// # };
    /// ```
    ///
    fn stream(
        &self,
        _input_variables: PromptArgs,
    ) -> impl Future<Output = Result<ChainStream, ChainError>> + Send {
        async {
            log::warn!("stream not implemented for this chain");
            unimplemented!()
        }
    }

    /// Returns the keys that must be present in input_variables for this chain.
    /// Default implementation returns empty — override in chains that have required keys.
    fn required_keys(&self) -> Vec<String> {
        vec![]
    }

    /// Validates that all required keys are present in input_variables.
    /// Returns `Err(ChainError::MissingInputVariable)` for the first missing key.
    fn validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError> {
        for key in self.required_keys() {
            if !input_variables.contains_key(&key) {
                return Err(ChainError::MissingInputVariable(key));
            }
        }
        Ok(())
    }

    // Get the input keys of the prompt
    fn get_input_keys(&self) -> Vec<String> {
        log::info!("Using default implementation");
        vec![]
    }

    fn get_output_keys(&self) -> Vec<String> {
        log::info!("Using default implementation");
        vec![
            String::from(DEFAULT_OUTPUT_KEY),
            String::from(DEFAULT_RESULT_KEY),
        ]
    }
}

pub type BoxChainFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait DynChain: Sync + Send {
    fn call(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<GenerateResult, ChainError>>;
    fn invoke(&self, input_variables: PromptArgs)
    -> BoxChainFuture<'_, Result<String, ChainError>>;
    fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<HashMap<String, Value>, ChainError>>;
    fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<ChainStream, ChainError>>;
    fn required_keys(&self) -> Vec<String>;
    fn validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError>;
    fn get_input_keys(&self) -> Vec<String>;
    fn get_output_keys(&self) -> Vec<String>;
}

impl<C: Chain> DynChain for C {
    fn call(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<GenerateResult, ChainError>> {
        Box::pin(Chain::call(self, input_variables))
    }

    fn invoke(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<String, ChainError>> {
        Box::pin(Chain::invoke(self, input_variables))
    }

    fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<HashMap<String, Value>, ChainError>> {
        Box::pin(Chain::execute(self, input_variables))
    }

    fn stream(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<ChainStream, ChainError>> {
        Box::pin(Chain::stream(self, input_variables))
    }

    fn required_keys(&self) -> Vec<String> {
        Chain::required_keys(self)
    }

    fn validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError> {
        Chain::validate_input(self, input_variables)
    }

    fn get_input_keys(&self) -> Vec<String> {
        Chain::get_input_keys(self)
    }

    fn get_output_keys(&self) -> Vec<String> {
        Chain::get_output_keys(self)
    }
}

impl<C> From<C> for Box<dyn DynChain>
where
    C: Chain + 'static,
{
    fn from(chain: C) -> Self {
        Box::new(chain)
    }
}
