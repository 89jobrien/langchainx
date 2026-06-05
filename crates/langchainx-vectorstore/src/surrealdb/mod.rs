mod builder;
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod surrealdb;

pub use builder::*;
pub use surrealdb::*;
