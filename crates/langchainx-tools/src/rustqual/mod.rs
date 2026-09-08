//! Rust code-quality analysis through the `rustqual` CLI.
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod rustqual;
pub use rustqual::RustqualTool;
