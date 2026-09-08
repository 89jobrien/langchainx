//! Embedding-based routing with configurable indexes and score aggregation.
#![deny(missing_docs)]
mod router;
pub use router::*;

mod route_layer;
pub use route_layer::*;

mod index;
pub use index::*;

pub mod utils;
