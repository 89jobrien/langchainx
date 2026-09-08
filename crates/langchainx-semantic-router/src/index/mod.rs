//! Index abstractions, in-memory storage, and related errors.
#[allow(clippy::module_inception)]
mod index;
pub use index::*;

mod memory_index;
pub use memory_index::*;

mod error;
pub use error::*;
