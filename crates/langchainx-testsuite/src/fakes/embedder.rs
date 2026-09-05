use async_trait::async_trait;
use langchainx_embedding::{Embedder, EmbedderError};

/// A deterministic embedder producing normalized vectors of a fixed size.
#[derive(Clone, Debug)]
pub struct FakeEmbedder {
    dimensions: usize,
}

impl FakeEmbedder {
    /// Creates an embedder whose vectors contain `dimensions` values.
    ///
    /// # Panics
    ///
    /// Panics when `dimensions` is zero because zero-length vectors violate the Embedder contract.
    pub fn new(dimensions: usize) -> Self {
        assert!(dimensions > 0, "FakeEmbedder dimensions must be nonzero");
        Self { dimensions }
    }

    fn embed_text(&self, text: &str) -> Vec<f64> {
        let hash = text.bytes().enumerate().fold(0_u64, |acc, (index, byte)| {
            acc.wrapping_add(u64::from(byte).wrapping_mul(index as u64 + 1))
        });
        let mut vector: Vec<f64> = (0..self.dimensions)
            .map(|index| {
                let value = hash.wrapping_add(index as u64) as f64;
                (value % 100.0) / 100.0 - 0.5
            })
            .collect();
        let norm = vector
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
            .max(1e-9);
        vector.iter_mut().for_each(|value| *value /= norm);
        vector
    }
}

#[async_trait]
impl Embedder for FakeEmbedder {
    async fn embed_documents(&self, documents: &[String]) -> Result<Vec<Vec<f64>>, EmbedderError> {
        Ok(documents
            .iter()
            .map(|document| self.embed_text(document))
            .collect())
    }

    async fn embed_query(&self, text: &str) -> Result<Vec<f64>, EmbedderError> {
        Ok(self.embed_text(text))
    }
}
