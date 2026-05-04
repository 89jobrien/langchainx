use serde_json::Value;
use std::io::{self, Write};

use crate::language_models::TokenUsage;

#[derive(Debug, Clone)]
pub struct StreamData {
    pub value: Value,
    pub tokens: Option<TokenUsage>,
    pub content: String,
}

impl StreamData {
    pub fn new<S: Into<String>>(value: Value, tokens: Option<TokenUsage>, content: S) -> Self {
        Self {
            value,
            tokens,
            content: content.into(),
        }
    }

    pub fn to_stdout(&self) -> io::Result<()> {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "{}", self.content)?;
        handle.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_sets_all_fields() {
        let sd = StreamData::new(json!("tok"), None, "hello");
        assert_eq!(sd.content, "hello");
        assert_eq!(sd.value, json!("tok"));
        assert!(sd.tokens.is_none());
    }

    #[test]
    fn new_with_tokens() {
        use crate::language_models::TokenUsage;
        let usage = TokenUsage::new(10, 5);
        let sd = StreamData::new(json!(null), Some(usage.clone()), "text");
        let t = sd.tokens.unwrap();
        assert_eq!(t.prompt_tokens, 10);
        assert_eq!(t.completion_tokens, 5);
        assert_eq!(t.total_tokens, 15);
    }
}
