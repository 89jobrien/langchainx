use thiserror::Error;

/// Errors produced by an MCP transport or protocol peer.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum McpClientError {
    /// The configured process could not be launched safely.
    #[error("MCP launch failed: {0}")]
    Launch(String),
    /// The peer sent an invalid or rejected protocol message.
    #[error("MCP protocol failure: {0}")]
    Protocol(String),
    /// A bounded operation exceeded its deadline.
    #[error("MCP operation timed out")]
    Timeout,
    /// A complete inbound or outbound frame exceeded its byte bound.
    #[error("MCP message exceeds {limit} bytes")]
    MessageTooLarge {
        /// Configured frame limit.
        limit: usize,
    },
    /// The peer or client has closed the connection.
    #[error("MCP client disconnected")]
    Disconnected,
    /// The monotonically increasing request identifier cannot advance safely.
    #[error("MCP request identifier exhausted")]
    RequestIdExhausted,
    /// A request identifier was already registered as pending.
    #[error("MCP request identifier collision")]
    RequestIdCollision,
    /// A configured limit is zero or exceeds the library ceiling.
    #[error("invalid MCP limit: {field}")]
    InvalidLimits {
        /// Name of the invalid limit field.
        field: &'static str,
    },
    /// Too many cancelled requests are awaiting possible late responses.
    #[error("MCP cancellation tracking exceeds {limit} request IDs")]
    CancellationLimitExceeded {
        /// Configured cancellation tombstone limit.
        limit: usize,
    },
}

/// Errors produced while validating and constructing an MCP toolkit.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpToolkitError {
    /// Tool discovery failed at the client boundary.
    #[error(transparent)]
    Client(#[from] McpClientError),
    /// Two server names normalize to the same exposed tool name.
    #[error("duplicate MCP tool name: {0}")]
    DuplicateToolName(String),
    /// A tool schema is unsafe or unsupported.
    #[error("invalid MCP tool schema: {0}")]
    InvalidToolSchema(String),
    /// Discovery returned more tools than permitted.
    #[error("MCP discovery exceeds {limit} tools")]
    TooManyTools {
        /// Configured tool count limit.
        limit: usize,
    },
    /// A discovery cursor was empty or repeated.
    #[error("MCP discovery cursor loop")]
    CursorLoop,
}
