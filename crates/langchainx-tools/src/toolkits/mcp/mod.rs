//! Tools discovered through bounded Model Context Protocol clients.

mod client;
mod error;
mod framing;
mod limits;
mod protocol;
mod stdio;
mod tool;
mod toolkit;

#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;

pub use client::{McpClient, McpToolDefinition, McpToolPage, McpToolResult};
pub use error::{McpClientError, McpToolkitError};
pub use limits::{McpStdioLimits, McpToolkitLimits};
pub use stdio::{StdioMcpClient, StdioMcpConfig};
pub use toolkit::McpToolkit;
