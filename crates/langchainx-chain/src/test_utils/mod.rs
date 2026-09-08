//! Deterministic model and temporary-workspace helpers for offline tests.
mod fake_llm;
pub use fake_llm::FakeLLM;

mod temp_workspace;
pub use temp_workspace::TempWorkspace;
