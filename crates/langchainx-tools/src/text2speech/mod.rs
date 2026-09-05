//! Text-to-speech clients and output storage abstraction.
pub mod openai;
pub use openai::*;

mod speech_storage;
pub use speech_storage::*;
