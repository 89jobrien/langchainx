//! Provider-neutral options for language-model calls.
use crate::schemas::{FunctionCallBehavior, FunctionDefinition, ResponseFormat};

#[derive(Clone)]
/// Optional generation settings applied to language-model requests.
pub struct CallOptions {
    /// Number of candidate responses to generate when supported.
    pub candidate_count: Option<usize>,
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Sequences that stop generation.
    pub stop_words: Option<Vec<String>>,
    /// Number of highest-probability tokens considered during sampling.
    pub top_k: Option<usize>,
    /// Cumulative probability threshold used for nucleus sampling.
    pub top_p: Option<f32>,
    /// Seed for deterministic sampling when supported.
    pub seed: Option<usize>,
    /// Minimum generated length when supported.
    pub min_length: Option<usize>,
    /// Maximum generated length when supported.
    pub max_length: Option<usize>,
    /// Number of completions to return when supported.
    pub n: Option<usize>,
    /// Penalty applied to repeated tokens.
    pub repetition_penalty: Option<f32>,
    /// Penalty based on how often tokens already appear.
    pub frequency_penalty: Option<f32>,
    /// Penalty for tokens that have already appeared.
    pub presence_penalty: Option<f32>,
    /// Function definitions available for tool calling.
    pub functions: Option<Vec<FunctionDefinition>>,
    /// Controls whether and which function may be called.
    pub function_call_behavior: Option<FunctionCallBehavior>,
    /// Required response representation.
    pub response_format: Option<ResponseFormat>,
    /// Whether streaming responses should include token usage.
    pub stream_usage: Option<bool>,
}

impl Default for CallOptions {
    fn default() -> Self {
        CallOptions::new()
    }
}
impl CallOptions {
    /// Creates options with every setting unset.
    pub fn new() -> Self {
        CallOptions {
            candidate_count: None,
            max_tokens: None,
            temperature: None,
            stop_words: None,
            top_k: None,
            top_p: None,
            seed: None,
            min_length: None,
            max_length: None,
            n: None,
            repetition_penalty: None,
            frequency_penalty: None,
            presence_penalty: None,
            functions: None,
            function_call_behavior: None,
            response_format: None,
            stream_usage: None,
        }
    }

    // Refactored "with" functions as methods of CallOptions
    /// Sets the maximum number of generated tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the requested number of candidate responses.
    pub fn with_candidate_count(mut self, candidate_count: usize) -> Self {
        self.candidate_count = Some(candidate_count);
        self
    }

    /// Sets the sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets sequences that stop generation.
    pub fn with_stop_words(mut self, stop_words: Vec<String>) -> Self {
        self.stop_words = Some(stop_words);
        self
    }

    /// Sets the top-k sampling limit.
    pub fn with_top_k(mut self, top_k: usize) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Sets the nucleus-sampling probability threshold.
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Sets the sampling seed.
    pub fn with_seed(mut self, seed: usize) -> Self {
        self.seed = Some(seed);
        self
    }

    /// Sets the minimum generated length.
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }

    /// Sets the maximum generated length.
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Sets the requested number of completions.
    pub fn with_n(mut self, n: usize) -> Self {
        self.n = Some(n);
        self
    }

    /// Sets the repetition penalty.
    pub fn with_repetition_penalty(mut self, repetition_penalty: f32) -> Self {
        self.repetition_penalty = Some(repetition_penalty);
        self
    }

    /// Sets the frequency penalty.
    pub fn with_frequency_penalty(mut self, frequency_penalty: f32) -> Self {
        self.frequency_penalty = Some(frequency_penalty);
        self
    }

    /// Sets the presence penalty.
    pub fn with_presence_penalty(mut self, presence_penalty: f32) -> Self {
        self.presence_penalty = Some(presence_penalty);
        self
    }

    /// Sets the function definitions available to the model.
    pub fn with_functions(mut self, functions: Vec<FunctionDefinition>) -> Self {
        self.functions = Some(functions);
        self
    }

    /// Sets function-call selection behavior.
    pub fn with_function_call_behavior(mut self, behavior: FunctionCallBehavior) -> Self {
        self.function_call_behavior = Some(behavior);
        self
    }

    /// Sets the required response format.
    pub fn with_response_format(mut self, response_format: ResponseFormat) -> Self {
        self.response_format = Some(response_format);
        self
    }

    /// Controls whether streamed responses include token usage.
    pub fn with_stream_usage(mut self, stream_usage: bool) -> Self {
        self.stream_usage = Some(stream_usage);
        self
    }

    /// Merges incoming settings, preferring incoming scalar values and appending lists.
    pub fn merge_options(&mut self, incoming_options: CallOptions) {
        // For simple scalar types wrapped in Option, prefer incoming option if it is Some
        self.candidate_count = incoming_options.candidate_count.or(self.candidate_count);
        self.max_tokens = incoming_options.max_tokens.or(self.max_tokens);
        self.temperature = incoming_options.temperature.or(self.temperature);
        self.top_k = incoming_options.top_k.or(self.top_k);
        self.top_p = incoming_options.top_p.or(self.top_p);
        self.seed = incoming_options.seed.or(self.seed);
        self.min_length = incoming_options.min_length.or(self.min_length);
        self.max_length = incoming_options.max_length.or(self.max_length);
        self.n = incoming_options.n.or(self.n);
        self.repetition_penalty = incoming_options
            .repetition_penalty
            .or(self.repetition_penalty);
        self.frequency_penalty = incoming_options
            .frequency_penalty
            .or(self.frequency_penalty);
        self.presence_penalty = incoming_options.presence_penalty.or(self.presence_penalty);
        self.function_call_behavior = incoming_options
            .function_call_behavior
            .or(self.function_call_behavior.clone());
        self.response_format = incoming_options
            .response_format
            .or(self.response_format.clone());
        self.stream_usage = incoming_options.stream_usage.or(self.stream_usage);

        // For `Vec<String>`, merge if both are Some; prefer incoming if only incoming is Some
        if let Some(mut new_stop_words) = incoming_options.stop_words {
            if let Some(existing_stop_words) = &mut self.stop_words {
                existing_stop_words.append(&mut new_stop_words);
            } else {
                self.stop_words = Some(new_stop_words);
            }
        }

        // For `Vec<FunctionDefinition>`, similar logic to `Vec<String>`
        if let Some(mut incoming_functions) = incoming_options.functions {
            if let Some(existing_functions) = &mut self.functions {
                existing_functions.append(&mut incoming_functions);
            } else {
                self.functions = Some(incoming_functions);
            }
        }
    }
}
