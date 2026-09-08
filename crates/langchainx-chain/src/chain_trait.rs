//! Shared execution interface implemented by all chains.
use std::{collections::HashMap, future::Future, pin::Pin};

use futures::Stream;
use serde_json::{Value, json};

use crate::{language_models::GenerateResult, prompt::PromptArgs, schemas::StreamData};

use super::ChainError;

pub(crate) const DEFAULT_OUTPUT_KEY: &str = "output";
pub(crate) const DEFAULT_RESULT_KEY: &str = "generate_result";
/// A sendable stream of chain output chunks.
pub type ChainStream = Pin<Box<dyn Stream<Item = Result<StreamData, ChainError>> + Send>>;

/// An asynchronous computation from named prompt inputs to a model generation.
pub trait Chain: Sync + Send {
    /// Runs the chain and returns generated text together with available token usage.
    fn call(
        &self,
        input_variables: PromptArgs,
    ) -> impl Future<Output = Result<GenerateResult, ChainError>> + Send;

    /// Runs the chain and returns only its generated text.
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

    /// Runs the chain and returns named outputs.
    ///
    /// The default implementation stores text under the first output key and the complete
    /// generation, including token usage, under `generate_result`.
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
    /// Starts an asynchronous stream of generation chunks.
    ///
    /// Stateful chains decide whether and when streamed content is committed to memory.
    ///
    /// # Panics
    ///
    /// The default implementation panics; streaming chains must override this method.
    fn stream(
        &self,
        _input_variables: PromptArgs,
    ) -> impl Future<Output = Result<ChainStream, ChainError>> + Send {
        async {
            log::warn!("stream not implemented for this chain");
            unimplemented!()
        }
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

/// A boxed, sendable future used at dynamic chain boundaries.
pub type BoxChainFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Object-safe adapter for dynamically dispatched [`Chain`] implementations.
pub trait DynChain: Sync + Send {
    /// Calls the wrapped chain through a boxed future.
    fn dyn_call(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<GenerateResult, ChainError>>;
    /// Invokes the wrapped chain through a boxed future.
    fn dyn_invoke(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<String, ChainError>>;
    /// Executes the wrapped chain through a boxed future.
    fn dyn_execute(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<HashMap<String, Value>, ChainError>>;
    /// Streams the wrapped chain through a boxed future.
    fn dyn_stream(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<ChainStream, ChainError>>;
    /// Returns the wrapped chain's required input keys.
    fn dyn_required_keys(&self) -> Vec<String>;
    /// Validates inputs using the wrapped chain.
    fn dyn_validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError>;
    /// Returns the wrapped chain's input keys.
    fn dyn_get_input_keys(&self) -> Vec<String>;
    /// Returns the wrapped chain's output keys.
    fn dyn_get_output_keys(&self) -> Vec<String>;
}

impl<C: Chain> DynChain for C {
    fn dyn_call(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<GenerateResult, ChainError>> {
        Box::pin(Chain::call(self, input_variables))
    }

    fn dyn_invoke(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<String, ChainError>> {
        Box::pin(Chain::invoke(self, input_variables))
    }

    fn dyn_execute(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<HashMap<String, Value>, ChainError>> {
        Box::pin(Chain::execute(self, input_variables))
    }

    fn dyn_stream(
        &self,
        input_variables: PromptArgs,
    ) -> BoxChainFuture<'_, Result<ChainStream, ChainError>> {
        Box::pin(Chain::stream(self, input_variables))
    }

    fn dyn_required_keys(&self) -> Vec<String> {
        Chain::required_keys(self)
    }

    fn dyn_validate_input(&self, input_variables: &PromptArgs) -> Result<(), ChainError> {
        Chain::validate_input(self, input_variables)
    }

    fn dyn_get_input_keys(&self) -> Vec<String> {
        Chain::get_input_keys(self)
    }

    fn dyn_get_output_keys(&self) -> Vec<String> {
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
