//! Cargo test and Clippy tool adapters.
pub mod clippy;
pub use clippy::ClippyTool;

pub mod cargo_test;
pub use cargo_test::CargoTestTool;
