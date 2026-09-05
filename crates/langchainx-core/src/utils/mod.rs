//! Shared type aliases and utility helpers used across langchainx crates.

/// Convenience alias for arbitrary key/value metadata attached to documents,
/// messages, and other langchainx types.
pub type Metadata = std::collections::HashMap<String, serde_json::Value>;
