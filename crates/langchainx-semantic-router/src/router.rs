//! Route definitions used by semantic indexes and route layers.
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
/// A named semantic route described by representative utterances.
pub struct Router {
    /// Unique route name used for identity and index lookup.
    pub name: String,
    /// Example inputs that describe the route's semantic intent.
    pub utterances: Vec<String>,
    /// Embedding vectors corresponding to the route's utterances.
    pub embedding: Option<Vec<Vec<f64>>>,
    /// Optional route-specific similarity value.
    pub similarity: Option<f64>,
    /// Optional tool description used to generate structured tool input.
    pub tool_description: Option<String>,
}
impl Router {
    /// Creates a route without embeddings, similarity, or a tool description.
    pub fn new<S: AsRef<str>>(name: &str, utterances: &[S]) -> Self {
        Self {
            name: name.into(),
            utterances: utterances.iter().map(|s| s.as_ref().to_string()).collect(),
            embedding: None,
            similarity: None,
            tool_description: None,
        }
    }

    /// Attaches precomputed utterance embeddings.
    pub fn with_embedding(mut self, embedding: Vec<Vec<f64>>) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Attaches a tool description for LLM-generated tool input.
    pub fn with_tool_description<S: Into<String>>(mut self, tool_description: S) -> Self {
        self.tool_description = Some(tool_description.into());
        self
    }

    /// Attaches a route-specific similarity value.
    pub fn with_similarity(mut self, similarity: f64) -> Self {
        self.similarity = Some(similarity);
        self
    }
}

impl Eq for Router {}
impl PartialEq for Router {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Hash for Router {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}
