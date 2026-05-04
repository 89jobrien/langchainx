//! Shared utility helpers used across `langchainx` crates.
//!
//! This module is intentionally sparse at first. It grows as extraction work
//! (issues #22–#27) surfaces duplication that belongs here rather than in
//! individual crates.

/// Convenience alias for arbitrary key/value metadata attached to documents,
/// messages, and other langchainx types.
pub type Metadata = std::collections::HashMap<String, serde_json::Value>;
