use std::sync::Arc;

use http::{HeaderMap, HeaderName};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::Tool;

use super::{
    HttpEgressPolicy, HttpOrigin, HttpTransport, OpenApiFormat, OpenApiLimits, OpenApiToolkitError,
    OperationPolicy,
    arguments::{is_forbidden_credential_header, is_protected_header},
    operation::{OpenApiOperation, ParameterLocation},
    spec::parse_operations,
    tool::OpenApiTool,
};

/// Builder for a bounded runtime OpenAPI toolkit.
pub struct OpenApiToolkitBuilder {
    specification: String,
    format: OpenApiFormat,
    transport: Arc<dyn HttpTransport>,
    egress_policy: Arc<dyn HttpEgressPolicy>,
    operation_policy: Option<Arc<dyn OperationPolicy>>,
    server_override: Option<Url>,
    credential_headers: HeaderMap<SecretString>,
    credential_origin: Option<HttpOrigin>,
    name_prefix: Option<String>,
    limits: OpenApiLimits,
}

impl OpenApiToolkitBuilder {
    /// Creates a builder with required specification and HTTP ports.
    pub fn new(
        specification: impl Into<String>,
        format: OpenApiFormat,
        transport: Arc<dyn HttpTransport>,
        egress_policy: Arc<dyn HttpEgressPolicy>,
    ) -> Self {
        Self {
            specification: specification.into(),
            format,
            transport,
            egress_policy,
            operation_policy: None,
            server_override: None,
            credential_headers: HeaderMap::<SecretString>::with_capacity(0),
            credential_origin: None,
            name_prefix: None,
            limits: OpenApiLimits::default(),
        }
    }

    /// Selects the explicit execution policy required by [`Self::build`].
    pub fn with_operation_policy(mut self, policy: Arc<dyn OperationPolicy>) -> Self {
        self.operation_policy = Some(policy);
        self
    }

    /// Replaces every server declared in the specification.
    pub fn with_server_override(mut self, server: Url) -> Self {
        self.server_override = Some(server);
        self
    }

    /// Adds a secret request header bound to one exact origin.
    ///
    /// # Errors
    ///
    /// Returns an error when the header controls proxying, framing, or transport behavior, when
    /// its value is invalid, when it duplicates another credential, or when its origin differs
    /// from credentials already configured on this builder.
    pub fn with_credential_header(
        mut self,
        origin: HttpOrigin,
        name: HeaderName,
        value: SecretString,
    ) -> Result<Self, OpenApiToolkitError> {
        if is_forbidden_credential_header(&name) {
            return Err(OpenApiToolkitError::ProtectedHeader(name.to_string()));
        }
        http::HeaderValue::from_str(value.expose_secret())
            .map_err(|_| OpenApiToolkitError::ProtectedHeader(name.to_string()))?;
        if self
            .credential_origin
            .as_ref()
            .is_some_and(|current| current != &origin)
        {
            return Err(OpenApiToolkitError::ProtectedHeader(
                "credentials span multiple origins".into(),
            ));
        }
        if self.credential_headers.contains_key(&name) {
            return Err(OpenApiToolkitError::ProtectedHeader(name.to_string()));
        }
        self.credential_origin = Some(origin);
        self.credential_headers.insert(name, value);
        Ok(self)
    }

    /// Prefixes every normalized generated tool name.
    pub fn with_name_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.name_prefix = Some(prefix.into());
        self
    }

    /// Replaces parser and HTTP limits, validated during build.
    pub fn with_limits(mut self, limits: OpenApiLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Validates the specification and creates generated operation tools.
    ///
    /// # Errors
    ///
    /// Returns an error when limits, policies, servers, credentials, operations, references, or
    /// schemas violate the bounded OpenAPI subset supported by this toolkit.
    pub fn build(self) -> Result<OpenApiToolkit, OpenApiToolkitError> {
        self.limits.validate()?;
        let operation_policy = self
            .operation_policy
            .ok_or(OpenApiToolkitError::MissingOperationPolicy)?;
        let operations = parse_operations(
            &self.specification,
            self.format,
            self.server_override.as_ref(),
            self.name_prefix.as_deref(),
            self.limits,
        )?;
        for operation in &operations {
            HttpOrigin::try_from_url(&operation.server)?;
        }
        if let Some(credential_origin) = &self.credential_origin {
            for operation in &operations {
                if HttpOrigin::try_from_url(&operation.server)? != *credential_origin {
                    return Err(OpenApiToolkitError::ProtectedHeader(
                        "credential origin does not match every operation".into(),
                    ));
                }
            }
        }
        for operation in &operations {
            if operation.parameters.iter().any(|parameter| {
                parameter.location == ParameterLocation::Cookie
                    && self.credential_headers.contains_key(http::header::COOKIE)
            }) {
                return Err(OpenApiToolkitError::ProtectedHeader("cookie".into()));
            }
            for parameter in &operation.parameters {
                if parameter.location == ParameterLocation::Header {
                    let name = HeaderName::from_bytes(parameter.name.as_bytes()).map_err(|_| {
                        OpenApiToolkitError::ProtectedHeader("invalid header name".into())
                    })?;
                    if is_protected_header(&name) || self.credential_headers.contains_key(&name) {
                        return Err(OpenApiToolkitError::ProtectedHeader(name.to_string()));
                    }
                }
            }
        }
        Ok(OpenApiToolkit {
            operations,
            transport: self.transport,
            egress_policy: self.egress_policy,
            operation_policy,
            credential_headers: self.credential_headers,
            limits: self.limits,
        })
    }
}

/// Validated collection of OpenAPI operation tools.
#[derive(Clone)]
pub struct OpenApiToolkit {
    pub(crate) operations: Vec<OpenApiOperation>,
    pub(crate) transport: Arc<dyn HttpTransport>,
    pub(crate) egress_policy: Arc<dyn HttpEgressPolicy>,
    pub(crate) operation_policy: Arc<dyn OperationPolicy>,
    pub(crate) credential_headers: HeaderMap<SecretString>,
    pub(crate) limits: OpenApiLimits,
}

impl OpenApiToolkit {
    /// Returns one independent adapter for every validated operation.
    pub fn tools(&self) -> Vec<Arc<dyn Tool>> {
        self.operations
            .iter()
            .cloned()
            .map(|operation| {
                Arc::new(OpenApiTool::new(
                    operation,
                    self.transport.clone(),
                    self.egress_policy.clone(),
                    self.operation_policy.clone(),
                    self.credential_headers.clone(),
                    self.limits,
                )) as Arc<dyn Tool>
            })
            .collect()
    }
}
