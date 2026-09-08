//! Rust model checking through the Kani CLI.
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod kani;
pub use kani::KaniTool;
