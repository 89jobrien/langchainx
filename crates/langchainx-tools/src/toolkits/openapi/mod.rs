//! Tools generated from runtime-validated OpenAPI specifications.

mod arguments;
mod builder;
mod error;
mod limits;
mod operation;
mod policy;
mod reqwest_transport;
mod spec;
mod tool;
mod transport;

#[cfg(feature = "fuzzing")]
#[doc(hidden)]
pub mod fuzzing;

pub use builder::{OpenApiToolkit, OpenApiToolkitBuilder};
pub use error::OpenApiToolkitError;
pub use limits::{HttpLimits, OpenApiLimits};
pub use policy::{
    AllowAllOperations, AuthorizedOrigin, HttpEgressPolicy, HttpOrigin, OperationContext,
    OperationPolicy, PublicHttpEgressPolicy, ReadOnlyOperations,
};
pub use reqwest_transport::ReqwestTransport;
pub use transport::{HttpBody, HttpRequest, HttpResponse, HttpTransport};

/// Input syntax used for an OpenAPI specification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenApiFormat {
    /// JavaScript Object Notation.
    Json,
    /// YAML Ain't Markup Language.
    Yaml,
}
