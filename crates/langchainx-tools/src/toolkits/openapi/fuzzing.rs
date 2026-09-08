//! Narrow entry points for fuzzing the production OpenAPI parser and argument mapper.

use http::HeaderMap;
use secrecy::SecretString;
use serde_json::Value;

use super::{
    HttpLimits, HttpOrigin, OpenApiFormat, OpenApiLimits, arguments::prepare_request,
    spec::parse_operations,
};

/// Bounds used by the OpenAPI fuzz harness.
pub const FUZZ_LIMITS: OpenApiLimits = OpenApiLimits {
    specification_bytes: 64 * 1024,
    operations: 16,
    schema_nodes: 2_048,
    reference_depth: 16,
    string_bytes: 4 * 1024,
    http: HttpLimits {
        request_body_bytes: 64 * 1024,
        response_body_bytes: 64 * 1024,
        url_bytes: 4 * 1024,
        header_bytes: 4 * 1024,
        timeout: std::time::Duration::from_secs(1),
    },
};

/// Observable bounded effects of parsing and mapping one fuzz input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenApiFuzzSummary {
    /// Number of operations accepted from the specification.
    pub operations: usize,
    /// Number of operations for which the supplied arguments produced a request.
    pub prepared_requests: usize,
    /// Largest generated URL in bytes.
    pub maximum_url_bytes: usize,
    /// Largest cumulative generated header size in bytes.
    pub maximum_header_bytes: usize,
    /// Largest generated body in bytes.
    pub maximum_body_bytes: usize,
    /// Whether every generated URL retained its specification server origin.
    pub origins_preserved: bool,
}

/// Runs arbitrary specification and JSON argument bytes through production code.
///
/// This entry point deliberately uses limits tighter than the runtime defaults. It exists only
/// behind the `fuzzing` feature and does not alter runtime validation or limits.
pub fn exercise_openapi(
    specification: &[u8],
    arguments: &[u8],
    format: OpenApiFormat,
) -> Result<OpenApiFuzzSummary, String> {
    let specification =
        std::str::from_utf8(specification).map_err(|_| "specification:invalid_utf8".to_owned())?;
    let operations = parse_operations(specification, format, None, None, FUZZ_LIMITS)
        .map_err(|error| format!("specification:{error}"))?;
    let input: Value =
        serde_json::from_slice(arguments).map_err(|_| "arguments:invalid_json".to_owned())?;
    let credentials = HeaderMap::<SecretString>::with_capacity(0);
    let mut summary = OpenApiFuzzSummary {
        operations: operations.len(),
        prepared_requests: 0,
        maximum_url_bytes: 0,
        maximum_header_bytes: 0,
        maximum_body_bytes: 0,
        origins_preserved: true,
    };

    for operation in &operations {
        let Ok(expected_origin) = HttpOrigin::try_from_url(&operation.server) else {
            summary.origins_preserved = false;
            continue;
        };
        let Ok(request) = prepare_request(operation, &input, &credentials, FUZZ_LIMITS.http) else {
            continue;
        };
        let Ok(actual_origin) = HttpOrigin::try_from_url(request.url()) else {
            summary.origins_preserved = false;
            continue;
        };
        summary.origins_preserved &= actual_origin == expected_origin;
        summary.prepared_requests += 1;
        summary.maximum_url_bytes = summary.maximum_url_bytes.max(request.url().as_str().len());
        let (header_bytes, body_bytes) = request.fuzz_sizes();
        summary.maximum_header_bytes = summary.maximum_header_bytes.max(header_bytes);
        summary.maximum_body_bytes = summary.maximum_body_bytes.max(body_bytes);
    }

    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exercises_each_operation_when_shared_arguments_are_rejected() {
        let specification = br#"{"openapi":"3.0.0","servers":[{"url":"https://x.example"}],"paths":{"/a":{"post":{"operationId":"needsBody","requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object"}}}},"responses":{}}},"/b":{"get":{"operationId":"acceptsEmpty","responses":{}}}}}"#;
        let summary = exercise_openapi(specification, b"{}", OpenApiFormat::Json).unwrap();
        assert_eq!(summary.operations, 2);
        assert_eq!(summary.prepared_requests, 1);
    }
}
