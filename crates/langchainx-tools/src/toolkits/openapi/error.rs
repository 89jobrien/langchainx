use thiserror::Error;

/// Errors produced while validating or executing an OpenAPI toolkit.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OpenApiToolkitError {
    /// The specification is malformed or exceeds a validation bound.
    #[error("invalid OpenAPI specification: {0}")]
    InvalidSpecification(String),
    /// The specification uses a construct this adapter cannot serialize safely.
    #[error("unsupported OpenAPI construct: {0}")]
    UnsupportedConstruct(String),
    /// No server URL is available for an operation.
    #[error("OpenAPI operation has no server")]
    MissingServer,
    /// An operation does not define the required operation identifier.
    #[error("OpenAPI operation {method} {path} has no operationId")]
    MissingOperationId {
        /// Canonical HTTP method.
        method: String,
        /// Specification path template.
        path: String,
    },
    /// Two operation identifiers normalize to the same tool name.
    #[error("duplicate OpenAPI tool name: {0}")]
    DuplicateToolName(String),
    /// Tool arguments do not match the generated schema.
    #[error("invalid OpenAPI arguments: {0}")]
    InvalidArguments(String),
    /// Toolkit construction omitted an execution policy.
    #[error("an explicit operation policy is required")]
    MissingOperationPolicy,
    /// The operation policy denied a mutating request.
    #[error("mutating OpenAPI operation denied: {0}")]
    MutatingOperationDenied(String),
    /// The egress policy denied the target origin.
    #[error("HTTP egress denied: {0}")]
    EgressDenied(String),
    /// A caller attempted to set a transport-controlled or credential header.
    #[error("protected HTTP header: {0}")]
    ProtectedHeader(String),
    /// The serialized request exceeds its configured byte bound.
    #[error("HTTP request exceeds {limit} bytes")]
    RequestTooLarge {
        /// Configured byte bound.
        limit: usize,
    },
    /// The response exceeds its configured byte bound.
    #[error("HTTP response exceeds {limit} bytes")]
    ResponseTooLarge {
        /// Configured byte bound.
        limit: usize,
    },
    /// The complete policy and HTTP operation exceeded its deadline.
    #[error("OpenAPI operation timed out")]
    Timeout,
    /// The HTTP adapter failed without exposing response data.
    #[error("HTTP transport failed: {0}")]
    Transport(String),
}
