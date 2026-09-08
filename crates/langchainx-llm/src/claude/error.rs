//! Errors reported by the Anthropic API.
use thiserror::Error;

#[derive(Error, Debug)]
/// Anthropic API error categories.
pub enum AnthropicError {
    #[error("Anthropic API error: Invalid request - {0}")]
    /// The request body or parameters were invalid.
    InvalidRequestError(String),

    #[error("Anthropic API error: Authentication failed - {0}")]
    /// Authentication credentials were missing or invalid.
    AuthenticationError(String),

    #[error("Anthropic API error: Permission denied - {0}")]
    /// The credentials lack permission for the requested operation.
    PermissionError(String),

    #[error("Anthropic API error: Not found - {0}")]
    /// The requested Anthropic resource was not found.
    NotFoundError(String),

    #[error("Anthropic API error: Rate limit exceeded - {0}")]
    /// The request exceeded an Anthropic rate limit.
    RateLimitError(String),

    #[error("Anthropic API error: Internal error - {0}")]
    /// Anthropic reported an internal API failure.
    ApiError(String),

    #[error("Anthropic API error: Overloaded - {0}")]
    /// Anthropic's service was temporarily overloaded.
    OverloadedError(String),
}
