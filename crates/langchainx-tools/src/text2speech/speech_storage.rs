//! Pluggable persistence for generated speech bytes.
use std::error::Error;

use async_trait::async_trait;

#[async_trait]
/// Stores generated audio and returns its resulting location or identifier.
pub trait SpeechStorage: Send + Sync {
    /// Saves `data` under `key` and returns the stored location or identifier.
    async fn save(&self, key: &str, data: &[u8]) -> Result<String, Box<dyn Error>>;
}
