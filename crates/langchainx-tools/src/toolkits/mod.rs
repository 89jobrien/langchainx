//! Adapters that expose third-party plugin operations as bounded tools.

#[cfg(any(feature = "openapi-toolkit", feature = "mcp-toolkit", test))]
mod common;

#[cfg(feature = "openapi-toolkit")]
pub mod openapi;

#[cfg(feature = "mcp-toolkit")]
pub mod mcp;
