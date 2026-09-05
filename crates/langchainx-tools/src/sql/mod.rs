//! Database inspection and query abstractions.
#[cfg(feature = "postgres")]
pub mod postgres;
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod sql;

pub use sql::*;
