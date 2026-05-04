#![allow(
    clippy::collapsible_if,
    clippy::from_over_into,
    clippy::get_first,
    clippy::into_iter_on_ref,
    clippy::manual_map,
    clippy::manual_range_contains,
    clippy::manual_strip,
    clippy::needless_borrows_for_generic_args,
    clippy::should_implement_trait,
    clippy::to_string_trait_impl,
    dead_code
)]

pub mod language_models;
pub mod schemas;

pub mod claude;
pub use claude::*;

pub mod deepseek;
pub use deepseek::*;

pub mod openai;
pub use openai::*;

pub mod qwen;
pub use qwen::*;

#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "ollama")]
pub use ollama::*;
