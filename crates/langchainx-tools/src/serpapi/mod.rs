#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod serpapi;
pub use serpapi::*;
