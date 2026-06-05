mod builder;
#[allow(clippy::module_inception)] // mod.rs + same-name child is the established pattern here
mod sqlite_vss;

pub use builder::*;
pub use sqlite_vss::*;
