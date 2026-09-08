//! Document content and retrieval metadata.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Text content with arbitrary metadata and a relevance score.
///
/// # Usage
/// ```rust,ignore
/// let my_doc = Document::new("This is the document content.".to_string())
///    .with_metadata({
///       let mut metadata = HashMap::new();
///       metadata.insert("author".to_string(), json!("John Doe"));
///       metadata
///   })
///    .with_score(0.75);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Text contained in the document.
    pub page_content: String,
    /// Arbitrary attributes associated with the document.
    pub metadata: HashMap<String, Value>,
    /// Relevance score assigned by a retriever or vector store.
    pub score: f64,
}

impl Document {
    /// Creates a document with empty metadata and a score of zero.
    pub fn new<S: Into<String>>(page_content: S) -> Self {
        Document {
            page_content: page_content.into(),
            metadata: HashMap::new(),
            score: 0.0,
        }
    }

    /// Replaces the document metadata.
    pub fn with_metadata(mut self, metadata: HashMap<String, Value>) -> Self {
        self.metadata = metadata;
        self
    }

    /// Sets the document's relevance score.
    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }
}

impl Default for Document {
    /// Creates an empty document with a score of zero.
    fn default() -> Self {
        Document {
            page_content: "".to_string(),
            metadata: HashMap::new(),
            score: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn new_sets_page_content() {
        let d = Document::new("hello world");
        assert_eq!(d.page_content, "hello world");
        assert!(d.metadata.is_empty());
        assert_eq!(d.score, 0.0);
    }

    #[test]
    fn with_metadata_sets_metadata() {
        let mut meta = HashMap::new();
        meta.insert("author".to_string(), json!("Alice"));
        let d = Document::new("content").with_metadata(meta);
        assert_eq!(d.metadata["author"], json!("Alice"));
    }

    #[test]
    fn with_score_sets_score() {
        let d = Document::new("x").with_score(0.85);
        assert!((d.score - 0.85).abs() < f64::EPSILON);
    }

    #[test]
    fn default_has_empty_content() {
        let d = Document::default();
        assert_eq!(d.page_content, "");
        assert_eq!(d.score, 0.0);
    }

    #[test]
    fn serde_round_trip() {
        let mut meta = HashMap::new();
        meta.insert("k".to_string(), json!("v"));
        let d = Document::new("text").with_metadata(meta).with_score(0.5);
        let json = serde_json::to_string(&d).unwrap();
        let restored: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.page_content, "text");
        assert_eq!(restored.metadata["k"], json!("v"));
        assert!((restored.score - 0.5).abs() < f64::EPSILON);
    }
}
