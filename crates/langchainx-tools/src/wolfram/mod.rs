//! Wolfram Alpha query tool.
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod wolfram;
pub use wolfram::*;
