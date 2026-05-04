use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::{
    chain::{Chain, ChainError, DEFAULT_OUTPUT_KEY, DEFAULT_RESULT_KEY},
    language_models::{GenerateResult, TokenUsage},
    prompt::PromptArgs,
};

//THIS IS EXPERIMENTAL
pub struct SequentialChain {
    pub(crate) chains: Vec<Box<dyn Chain>>,
    pub(crate) input_keys: HashSet<String>,
    pub(crate) outputs: HashSet<String>,
}

#[async_trait]
impl Chain for SequentialChain {
    async fn call(&self, input_variables: PromptArgs) -> Result<GenerateResult, ChainError> {
        let output = self.execute(input_variables).await?;
        let result = output
            .get(DEFAULT_RESULT_KEY)
            .ok_or_else(|| ChainError::MissingInputVariable(DEFAULT_RESULT_KEY.to_string()))?
            .clone();
        let result: GenerateResult = serde_json::from_value(result)?;
        Ok(result)
    }
    async fn invoke(&self, input_variables: PromptArgs) -> Result<String, ChainError> {
        self.call(input_variables.clone())
            .await
            .map(|result| result.generation)
    }
    fn get_input_keys(&self) -> Vec<String> {
        self.outputs.iter().cloned().collect()
    }

    async fn execute(
        &self,
        input_variables: PromptArgs,
    ) -> Result<HashMap<String, Value>, ChainError> {
        let mut input_variables = input_variables;
        let mut final_token_usage: Option<TokenUsage> = None;
        let mut output_result = HashMap::new();
        let mut final_result = GenerateResult::default();
        for chain in self.chains.iter() {
            let output = chain.execute(input_variables.clone()).await?;
            //Get the oput key for the chain result
            let output_key = chain
                .get_output_keys()
                .first()
                .unwrap_or(&DEFAULT_OUTPUT_KEY.to_string())
                .clone();
            //Get the ouput complete result
            let result = output
                .get(DEFAULT_RESULT_KEY)
                .unwrap_or(&json!(GenerateResult::default()))
                .clone();
            let result: GenerateResult = serde_json::from_value(result)?;
            log::debug!("{}", result.generation);
            //Insert the output chain to the final output
            output_result.insert(output_key.clone(), json!(result.generation.clone()));
            input_variables.insert(output_key, json!(result.generation.clone()));

            //add the generation to keep track of the final generation
            final_result.generation = result.generation;
            //Add to the token if it exist
            if let Some(token) = &result.tokens {
                match final_token_usage {
                    Some(token_usage) => {
                        final_token_usage = Some(token_usage.sum(token));
                    }
                    None => {
                        final_token_usage = Some(token.clone());
                    }
                }
            }
        }

        //add the filan token count to the result
        final_result.tokens = final_token_usage;
        output_result.insert(DEFAULT_RESULT_KEY.to_string(), json!(final_result));
        Ok(output_result)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        chain::{Chain, LLMChainBuilder},
        prompt_args, sequential_chain, template_fstring,
        test_utils::FakeLLM,
    };

    #[tokio::test]
    async fn sequential_chain_passes_output_as_next_input() {
        let chain1 = LLMChainBuilder::new()
            .prompt(template_fstring!("step one: {input}", "input"))
            .llm(FakeLLM::new(vec!["result_one".into()]))
            .output_key("step1_out")
            .build()
            .expect("failed to build chain1");

        let chain2 = LLMChainBuilder::new()
            .prompt(template_fstring!("step two: {step1_out}", "step1_out"))
            .llm(FakeLLM::new(vec!["result_two".into()]))
            .output_key("step2_out")
            .build()
            .expect("failed to build chain2");

        let seq = sequential_chain!(chain1, chain2);
        let output = seq
            .execute(prompt_args! { "input" => "start" })
            .await
            .expect("sequential chain failed");

        // Final generation should be from the last chain
        assert_eq!(
            output
                .get("step2_out")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
            "result_two"
        );
    }

    #[tokio::test]
    async fn sequential_chain_invoke_returns_last_generation() {
        let chain1 = LLMChainBuilder::new()
            .prompt(template_fstring!("{input}", "input"))
            .llm(FakeLLM::new(vec!["middle".into()]))
            .output_key("mid")
            .build()
            .unwrap();

        let chain2 = LLMChainBuilder::new()
            .prompt(template_fstring!("{mid}", "mid"))
            .llm(FakeLLM::new(vec!["final".into()]))
            .output_key("out")
            .build()
            .unwrap();

        let seq = sequential_chain!(chain1, chain2);
        let result = seq
            .invoke(prompt_args! { "input" => "go" })
            .await
            .expect("invoke failed");

        assert_eq!(result, "final");
    }
}
