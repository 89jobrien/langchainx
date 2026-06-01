use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GenerateResult {
    pub tokens: Option<TokenUsage>,
    pub generation: String,
}

impl GenerateResult {
    pub fn to_hashmap(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        // Insert the 'generation' field into the hashmap
        map.insert("generation".to_string(), self.generation.clone());

        // Check if 'tokens' is Some and insert its fields into the hashmap
        if let Some(ref tokens) = self.tokens {
            map.insert(
                "prompt_tokens".to_string(),
                tokens.prompt_tokens.to_string(),
            );
            map.insert(
                "completion_tokens".to_string(),
                tokens.completion_tokens.to_string(),
            );
            map.insert("total_tokens".to_string(), tokens.total_tokens.to_string());
        }

        map
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl TokenUsage {
    pub fn sum(&self, other: &TokenUsage) -> TokenUsage {
        TokenUsage {
            prompt_tokens: self.prompt_tokens.saturating_add(other.prompt_tokens),
            completion_tokens: self
                .completion_tokens
                .saturating_add(other.completion_tokens),
            total_tokens: self.total_tokens.saturating_add(other.total_tokens),
        }
    }

    pub fn add(&mut self, other: &TokenUsage) {
        self.prompt_tokens = self.prompt_tokens.saturating_add(other.prompt_tokens);
        self.completion_tokens = self
            .completion_tokens
            .saturating_add(other.completion_tokens);
        self.total_tokens = self.total_tokens.saturating_add(other.total_tokens);
    }
}

impl TokenUsage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_usage_sum_saturates() {
        let a = TokenUsage {
            prompt_tokens: u32::MAX,
            completion_tokens: u32::MAX,
            total_tokens: u32::MAX,
        };
        let b = TokenUsage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 1,
        };
        let result = a.sum(&b);
        assert_eq!(result.prompt_tokens, u32::MAX);
        assert_eq!(result.completion_tokens, u32::MAX);
        assert_eq!(result.total_tokens, u32::MAX);
    }

    #[test]
    fn token_usage_add_saturates() {
        let mut a = TokenUsage {
            prompt_tokens: u32::MAX,
            completion_tokens: u32::MAX,
            total_tokens: u32::MAX,
        };
        let b = TokenUsage {
            prompt_tokens: 1,
            completion_tokens: 1,
            total_tokens: 1,
        };
        a.add(&b);
        assert_eq!(a.prompt_tokens, u32::MAX);
        assert_eq!(a.completion_tokens, u32::MAX);
        assert_eq!(a.total_tokens, u32::MAX);
    }

    #[test]
    fn token_usage_new_saturates() {
        let t = TokenUsage::new(u32::MAX, 1);
        assert_eq!(t.total_tokens, u32::MAX);
    }
}
