//! Errors reported by the Qwen API.
use thiserror::Error;

#[derive(Error, Debug)]
/// Qwen API error categories.
pub enum QwenError {
    #[error("Qwen API error: Invalid parameter - {0}")]
    /// A request parameter was invalid.
    InvalidParameterError(String),

    #[error("Qwen API error: Invalid API Key - {0}")]
    /// The API key was missing or invalid.
    InvalidApiKeyError(String),

    #[error("Qwen API error: Network error - {0}")]
    /// A network operation failed.
    NetworkError(String),

    #[error("Qwen API error: Model Unavailable - {0}")]
    /// The requested model was temporarily unavailable.
    ModelUnavailableError(String),

    #[error("Qwen API error: Rate limit exceeded - {0}")]
    /// The model-serving service rejected or failed the request.
    ModelServingError(String),

    #[error("Qwen API error: Internal error - {0}")]
    /// Qwen reported an internal error.
    InternalError(String),

    #[error("Qwen API error: System error - {0}")]
    /// Qwen reported a system-level error.
    SystemError(String),

    #[error("Qwen API error: Billing issue - {0}")]
    /// Billing prevented the request from being processed.
    BillingError(String),

    #[error("Qwen API error: Mismatched model - {0}")]
    /// The request and selected model were incompatible.
    MismatchedModelError(String),

    #[error("Qwen API error: Duplicate custom ID - {0}")]
    /// A custom request identifier was duplicated.
    DuplicateCustomIdError(String),

    #[error("Qwen API error: Model not found - {0}")]
    /// The requested model could not be found.
    ModelNotFoundError(String),

    #[error("Qwen API error: Connection error - {0}")]
    /// The client could not connect to the API.
    APIConnectionError(String),

    #[error("Qwen API error: Prepaid bill overdue - {0}")]
    /// A prepaid account has an overdue balance.
    PrepaidBillOverdueError(String),

    #[error("Qwen API error: Postpaid bill overdue - {0}")]
    /// A postpaid account has an overdue balance.
    PostpaidBillOverdueError(String),

    #[error("Qwen API error: Commodity not purchased - {0}")]
    /// The required model or service has not been purchased.
    CommodityNotPurchasedError(String),

    #[error("Qwen API error: Internal algorithm error - {0}")]
    /// Qwen's internal inference algorithm failed.
    InternalAlgorithmError(String),

    #[error("Qwen API error: Timeout - {0}")]
    /// Qwen timed out while processing the request.
    TimeoutError(String),

    #[error("Qwen API error: Rewrite failed - {0}")]
    /// Qwen failed to rewrite the request.
    RewriteFailedError(String),

    #[error("Qwen API error: Retrieval failed - {0}")]
    /// Qwen failed to retrieve required data.
    RetrievalFailedError(String),

    #[error("Qwen API error: Application process failed - {0}")]
    /// Application-level request processing failed.
    AppProcessFailedError(String),

    #[error("Qwen API error: Model service failed - {0}")]
    /// The model service failed while processing the request.
    ModelServiceFailedError(String),

    #[error("Qwen API error: Plugin invocation failed - {0}")]
    /// A plugin invoked by Qwen failed.
    InvokePluginFailedError(String),
}
