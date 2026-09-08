//! Model generation options applied when a chain is built.
use crate::language_models::options::CallOptions;

/// Optional generation parameters forwarded from a chain to its language model.
pub struct ChainCallOptions {
    /// Maximum number of tokens to generate.
    pub max_tokens: Option<u32>,
    /// Sampling temperature.
    pub temperature: Option<f32>,
    /// Sequences that stop generation when encountered.
    pub stop_words: Option<Vec<String>>,
    /// Number of highest-probability tokens considered during sampling.
    pub top_k: Option<usize>,
    /// Cumulative probability threshold used for nucleus sampling.
    pub top_p: Option<f32>,
    /// Seed used for deterministic sampling when supported.
    pub seed: Option<usize>,
    /// Minimum generated sequence length.
    pub min_length: Option<usize>,
    /// Maximum generated sequence length.
    pub max_length: Option<usize>,
    /// Penalty applied to repeated tokens.
    pub repetition_penalty: Option<f32>,
}

impl Default for ChainCallOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainCallOptions {
    /// Creates options with every model parameter unset.
    pub fn new() -> Self {
        Self {
            max_tokens: None,
            temperature: None,
            stop_words: None,
            top_k: None,
            top_p: None,
            seed: None,
            min_length: None,
            max_length: None,
            repetition_penalty: None,
        }
    }

    /// Converts the configured values into language-model call options.
    pub fn to_llm_options(options: ChainCallOptions) -> CallOptions {
        let mut llm_option = CallOptions::new();
        if let Some(max_tokens) = options.max_tokens {
            llm_option = llm_option.with_max_tokens(max_tokens);
        }
        if let Some(temperature) = options.temperature {
            llm_option = llm_option.with_temperature(temperature);
        }
        if let Some(stop_words) = options.stop_words {
            llm_option = llm_option.with_stop_words(stop_words);
        }
        if let Some(top_k) = options.top_k {
            llm_option = llm_option.with_top_k(top_k);
        }
        if let Some(top_p) = options.top_p {
            llm_option = llm_option.with_top_p(top_p);
        }
        if let Some(seed) = options.seed {
            llm_option = llm_option.with_seed(seed);
        }
        if let Some(min_length) = options.min_length {
            llm_option = llm_option.with_min_length(min_length);
        }
        if let Some(max_length) = options.max_length {
            llm_option = llm_option.with_max_length(max_length);
        }
        if let Some(repetition_penalty) = options.repetition_penalty {
            llm_option = llm_option.with_repetition_penalty(repetition_penalty);
        }
        llm_option
    }

    /// Sets the maximum number of generated tokens.
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Sets the sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Sets the sequences that stop generation.
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

    /// Sets the minimum generated sequence length.
    pub fn with_min_length(mut self, min_length: usize) -> Self {
        self.min_length = Some(min_length);
        self
    }

    /// Sets the maximum generated sequence length.
    pub fn with_max_length(mut self, max_length: usize) -> Self {
        self.max_length = Some(max_length);
        self
    }

    /// Sets the penalty applied to repeated tokens.
    pub fn with_repetition_penalty(mut self, repetition_penalty: f32) -> Self {
        self.repetition_penalty = Some(repetition_penalty);
        self
    }
}
