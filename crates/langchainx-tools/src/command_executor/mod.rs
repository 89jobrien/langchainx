//! Sequential subprocess execution tool.
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod command_executor;
pub use command_executor::*;
