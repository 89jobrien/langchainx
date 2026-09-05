//! Shared execution interface implemented by all chains.
use std::{collections::HashMap, pin::Pin};

use async_trait::async_trait;
use futures::Stream;
use serde_json::{Value, json};

use crate::{language_models::GenerateResult, prompt::PromptArgs, schemas::StreamData};

use super::ChainError;

pub(crate) const DEFAULT_OUTPUT_KEY: &str = "output";
pub(crate) const DEFAULT_RESULT_KEY: &str = "generate_result";

#[async_trait]
/// An asynchronous computation from named prompt inputs to a model generation.
pub trait Chain: Sync + Send {
    /// Runs the chain and returns generated text together with available token usage.
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError>;

    /// Runs the chain and returns only its generated text.
    async fn invoke(&self, input_variables: PromptArgs) -> Result<String, ChainError> {
        self.call(input_variables)
            .await
            .map(|result| result.generation)
    }

    /// Runs the chain and returns named outputs.
    ///
    /// The default implementation stores text under the first output key and the complete
    /// generation, including token usage, under `generate_result`.
    async fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> Result<HashMap<String, Value>, ChainError> {
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
    /// Starts an asynchronous stream of generation chunks.
    ///
    /// Stateful chains decide whether and when streamed content is committed to memory.
    ///
    /// # Panics
    ///
    /// The default implementation panics; streaming chains must override this method.
    async fn stream(
        &self,
        _input_variables: PromptArgs,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>, ChainError>
    {
        log::warn!("stream not implemented for this chain");
        unimplemented!()
    }

    /// Returns the input keys that must be present before this chain can run.
    ///
    /// The default implementation accepts any input set.
    fn required_keys(&self) -> Vec<String> {
        vec![]
    }

    /// Validates that every [`required_keys`](Self::required_keys) entry is present.
    ///
    /// Returns [`ChainError::MissingInputVariable`] for the first missing key.
    fn validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError> {
        let required = self.required_keys();
        for key in &required {
            if !input_variables.contains_key(key) {
                return Err(ChainError::MissingInputVariable {
                    key: key.clone(),
                    expected: required.clone(),
                    provided: input_variables.keys().cloned().collect(),
                });
            }
        }
        Ok(())
    }

    /// Returns the input keys consumed by this chain.
    fn get_input_keys(&self) -> Vec<String> {
        log::info!("Using default implementation");
        vec![]
    }

    /// Returns the keys produced by [`execute`](Self::execute).
    fn get_output_keys(&self) -> Vec<String> {
        log::info!("Using default implementation");
        vec![
            String::from(DEFAULT_OUTPUT_KEY),
            String::from(DEFAULT_RESULT_KEY),
        ]
    }
}

impl<C> From<C> for Box<dyn Chain>
where
    C: Chain + 'static,
{
    fn from(chain: C) -> Self {
        Box::new(chain)
    }
}
