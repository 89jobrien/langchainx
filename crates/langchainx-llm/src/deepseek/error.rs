//! Errors reported by the DeepSeek API.
use thiserror::Error;

#[derive(Error, Debug)]
/// DeepSeek API error categories.
pub enum DeepseekError {
    #[error("Deepseek API error: Invalid Format - {0}")]
    /// The request format was invalid.
    InvalidFormatError(String),

    #[error("Deepseek API error: Authentication Failed - {0}")]
    /// Authentication credentials were invalid.
    AuthenticationError(String),

    #[error("Deepseek API error: Insufficient Balance - {0}")]
    /// The account lacks sufficient balance for the request.
    InsufficientBalanceError(String),

    #[error("Deepseek API error: Invalid Parameters - {0}")]
    /// One or more request parameters were invalid.
    InvalidParametersError(String),

    #[error("Deepseek API error: Rate Limit Reached - {0}")]
    /// The request exceeded a DeepSeek rate limit.
    RateLimitError(String),

    #[error("Deepseek API error: Server Error - {0}")]
    /// DeepSeek reported an internal server error.
    ServerError(String),

    #[error("Deepseek API error: Server Overloaded - {0}")]
    /// DeepSeek's service was temporarily overloaded.
    ServerOverloadedError(String),
}
