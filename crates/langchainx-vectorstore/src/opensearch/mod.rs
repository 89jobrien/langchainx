//! OpenSearch k-NN vector store and builder.
mod builder;
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod opensearch;

pub use builder::*;
pub use opensearch::*;
