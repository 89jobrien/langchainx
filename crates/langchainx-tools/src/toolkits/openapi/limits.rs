use std::time::Duration;

use super::OpenApiToolkitError;

const MAX_SPECIFICATION_BYTES: usize = 8 * 1024 * 1024;
const MAX_OPERATIONS: usize = 2_048;
const MAX_SCHEMA_NODES: usize = 100_000;
const MAX_REFERENCE_DEPTH: usize = 64;
const MAX_STRING_BYTES: usize = 1024 * 1024;
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 64 * 1024 * 1024;
const MAX_URL_BYTES: usize = 16 * 1024;
const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_TIMEOUT: Duration = Duration::from_secs(300);

/// Byte and duration bounds applied to each generated HTTP request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpLimits {
    /// Maximum serialized request body size.
    pub request_body_bytes: usize,
    /// Maximum post-decompression response body size.
    pub response_body_bytes: usize,
    /// Maximum final URL size.
    pub url_bytes: usize,
    /// Maximum cumulative request or response header size.
    pub header_bytes: usize,
    /// Deadline shared by policy checks, transport, and response consumption.
    pub timeout: Duration,
}

impl Default for HttpLimits {
    fn default() -> Self {
        Self {
            request_body_bytes: 1024 * 1024,
            response_body_bytes: 4 * 1024 * 1024,
            url_bytes: 8 * 1024,
            header_bytes: 32 * 1024,
            timeout: Duration::from_secs(30),
        }
    }
}

impl HttpLimits {
    pub(crate) fn validate(self) -> Result<(), OpenApiToolkitError> {
        validate_bound(
            "request_body_bytes",
            self.request_body_bytes,
            MAX_REQUEST_BODY_BYTES,
        )?;
        validate_bound(
            "response_body_bytes",
            self.response_body_bytes,
            MAX_RESPONSE_BODY_BYTES,
        )?;
        validate_bound("url_bytes", self.url_bytes, MAX_URL_BYTES)?;
        validate_bound("header_bytes", self.header_bytes, MAX_HEADER_BYTES)?;
        if self.timeout.is_zero() || self.timeout > MAX_TIMEOUT {
            return Err(OpenApiToolkitError::InvalidSpecification(
                "HTTP timeout is outside library bounds".into(),
            ));
        }
        Ok(())
    }
}

/// Bounds applied while parsing a specification and generating operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenApiLimits {
    /// Maximum source specification size.
    pub specification_bytes: usize,
    /// Maximum number of generated operations.
    pub operations: usize,
    /// Maximum number of traversed JSON or YAML nodes.
    pub schema_nodes: usize,
    /// Maximum local-reference traversal depth.
    pub reference_depth: usize,
    /// Maximum size of any individual string.
    pub string_bytes: usize,
    /// Per-request HTTP limits.
    pub http: HttpLimits,
}

impl Default for OpenApiLimits {
    fn default() -> Self {
        Self {
            specification_bytes: 2 * 1024 * 1024,
            operations: 512,
            schema_nodes: 25_000,
            reference_depth: 32,
            string_bytes: 256 * 1024,
            http: HttpLimits::default(),
        }
    }
}

impl OpenApiLimits {
    pub(crate) fn validate(self) -> Result<(), OpenApiToolkitError> {
        validate_bound(
            "specification_bytes",
            self.specification_bytes,
            MAX_SPECIFICATION_BYTES,
        )?;
        validate_bound("operations", self.operations, MAX_OPERATIONS)?;
        validate_bound("schema_nodes", self.schema_nodes, MAX_SCHEMA_NODES)?;
        validate_bound("reference_depth", self.reference_depth, MAX_REFERENCE_DEPTH)?;
        validate_bound("string_bytes", self.string_bytes, MAX_STRING_BYTES)?;
        self.http.validate()
    }
}

fn validate_bound(name: &str, value: usize, ceiling: usize) -> Result<(), OpenApiToolkitError> {
    if value == 0 || value > ceiling {
        return Err(OpenApiToolkitError::InvalidSpecification(format!(
            "{name} is outside library bounds"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_and_over_ceiling_limits() {
        let limits = OpenApiLimits {
            operations: 0,
            ..OpenApiLimits::default()
        };
        assert!(limits.validate().is_err());
        let limits = OpenApiLimits {
            operations: MAX_OPERATIONS + 1,
            ..OpenApiLimits::default()
        };
        assert!(limits.validate().is_err());
        let limits = OpenApiLimits {
            http: HttpLimits {
                timeout: Duration::ZERO,
                ..HttpLimits::default()
            },
            ..OpenApiLimits::default()
        };
        assert!(limits.validate().is_err());
    }
}
