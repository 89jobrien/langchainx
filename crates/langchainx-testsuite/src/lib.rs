//! Reusable contract assertions and deterministic fakes for langchainx implementations.
//!
//! Downstream crates can register their implementations against the shared contracts without
//! copying assertion logic or contacting external services.
#![deny(missing_docs)]

/// Assertions describing the public behavioral contracts of langchainx traits.
pub mod contracts;
/// Deterministic, network-free implementations for conformance and integration tests.
pub mod fakes;
