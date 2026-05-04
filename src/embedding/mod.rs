// Re-exported from langchainx-embedding crate.
pub use langchainx_embedding::embedding::embedder_trait;
pub use langchainx_embedding::embedding::*;

#[cfg(feature = "ollama")]
pub mod ollama {
    pub use langchainx_embedding::embedding::ollama::*;
}

pub mod openai {
    pub use langchainx_embedding::embedding::openai::*;
}

#[cfg(feature = "fastembed")]
pub mod fastembed {
    pub use langchainx_embedding::fastembed::*;
}

#[cfg(feature = "mistralai")]
pub mod mistralai {
    pub use langchainx_embedding::embedding::mistralai::*;
}
