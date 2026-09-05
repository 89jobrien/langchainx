use std::time::Duration;

use super::McpClientError;

const MAX_TOOLS: usize = 2_048;
const MAX_DISCOVERY_PAGES: usize = 256;
const MAX_DISCOVERY_BYTES: usize = 32 * 1024 * 1024;
const MAX_CURSOR_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_BYTES: usize = 2 * 1024 * 1024;
const MAX_SCHEMA_DEPTH: usize = 64;
const MAX_SCHEMA_NODES: usize = 100_000;
const MAX_ARGUMENT_BYTES: usize = 16 * 1024 * 1024;
const MAX_RESULT_BYTES: usize = 64 * 1024 * 1024;
const MAX_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;
const MAX_IN_FLIGHT: usize = 1_024;
const MAX_CANCELLED_REQUEST_IDS: usize = 65_536;
const MAX_REQUEST_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_CLOSE_TIMEOUT: Duration = Duration::from_secs(60);

/// Limits applied during MCP discovery and generated tool execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpToolkitLimits {
    /// Maximum total discovered tools.
    pub tools: usize,
    /// Maximum discovery pages.
    pub discovery_pages: usize,
    /// Maximum cumulative serialized discovery bytes.
    pub discovery_bytes: usize,
    /// Maximum bytes in one cursor.
    pub cursor_bytes: usize,
    /// Maximum serialized bytes in one input schema.
    pub schema_bytes: usize,
    /// Maximum nesting depth of one input schema.
    pub schema_depth: usize,
    /// Maximum traversed nodes in one input schema.
    pub schema_nodes: usize,
    /// Maximum serialized tool argument bytes.
    pub argument_bytes: usize,
    /// Maximum serialized tool result bytes.
    pub result_bytes: usize,
    /// Deadline shared by all discovery pages.
    pub discovery_timeout: Duration,
}

impl Default for McpToolkitLimits {
    fn default() -> Self {
        Self {
            tools: 256,
            discovery_pages: 32,
            discovery_bytes: 4 * 1024 * 1024,
            cursor_bytes: 4 * 1024,
            schema_bytes: 256 * 1024,
            schema_depth: 32,
            schema_nodes: 25_000,
            argument_bytes: 1024 * 1024,
            result_bytes: 4 * 1024 * 1024,
            discovery_timeout: Duration::from_secs(30),
        }
    }
}

impl McpToolkitLimits {
    pub(crate) fn validate(self) -> Result<(), McpClientError> {
        validate_bound("tools", self.tools, MAX_TOOLS)?;
        validate_bound("discovery_pages", self.discovery_pages, MAX_DISCOVERY_PAGES)?;
        validate_bound("discovery_bytes", self.discovery_bytes, MAX_DISCOVERY_BYTES)?;
        validate_bound("cursor_bytes", self.cursor_bytes, MAX_CURSOR_BYTES)?;
        validate_bound("schema_bytes", self.schema_bytes, MAX_SCHEMA_BYTES)?;
        validate_bound("schema_depth", self.schema_depth, MAX_SCHEMA_DEPTH)?;
        validate_bound("schema_nodes", self.schema_nodes, MAX_SCHEMA_NODES)?;
        validate_bound("argument_bytes", self.argument_bytes, MAX_ARGUMENT_BYTES)?;
        validate_bound("result_bytes", self.result_bytes, MAX_RESULT_BYTES)?;
        validate_duration(
            "discovery_timeout",
            self.discovery_timeout,
            MAX_DISCOVERY_TIMEOUT,
        )
    }
}

/// Limits applied to an MCP stdio connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpStdioLimits {
    /// Maximum bytes in one inbound newline-delimited frame.
    pub inbound_frame_bytes: usize,
    /// Maximum bytes in one outbound newline-delimited frame.
    pub outbound_frame_bytes: usize,
    /// Maximum requests concurrently awaiting responses.
    pub max_in_flight: usize,
    /// Maximum cancelled request IDs retained until their late responses arrive.
    pub cancelled_request_ids: usize,
    /// Deadline for a request and response exchange.
    pub request_timeout: Duration,
    /// Deadline for graceful shutdown before forceful termination.
    pub close_timeout: Duration,
}

impl Default for McpStdioLimits {
    fn default() -> Self {
        Self {
            inbound_frame_bytes: 4 * 1024 * 1024,
            outbound_frame_bytes: 4 * 1024 * 1024,
            max_in_flight: 16,
            cancelled_request_ids: 1_024,
            request_timeout: Duration::from_secs(30),
            close_timeout: Duration::from_secs(5),
        }
    }
}

impl McpStdioLimits {
    pub(crate) fn validate(self) -> Result<(), McpClientError> {
        validate_bound(
            "inbound_frame_bytes",
            self.inbound_frame_bytes,
            MAX_FRAME_BYTES,
        )?;
        validate_bound(
            "outbound_frame_bytes",
            self.outbound_frame_bytes,
            MAX_FRAME_BYTES,
        )?;
        validate_bound("max_in_flight", self.max_in_flight, MAX_IN_FLIGHT)?;
        validate_bound(
            "cancelled_request_ids",
            self.cancelled_request_ids,
            MAX_CANCELLED_REQUEST_IDS,
        )?;
        validate_duration("request_timeout", self.request_timeout, MAX_REQUEST_TIMEOUT)?;
        validate_duration("close_timeout", self.close_timeout, MAX_CLOSE_TIMEOUT)
    }
}

fn validate_bound(field: &'static str, value: usize, ceiling: usize) -> Result<(), McpClientError> {
    if value == 0 || value > ceiling {
        return Err(McpClientError::InvalidLimits { field });
    }
    Ok(())
}

fn validate_duration(
    field: &'static str,
    value: Duration,
    ceiling: Duration,
) -> Result<(), McpClientError> {
    if value.is_zero() || value > ceiling {
        return Err(McpClientError::InvalidLimits { field });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolkit_limits_reject_each_zero_field() {
        let default = McpToolkitLimits::default();
        assert!(default.validate().is_ok());
        for invalid in [
            McpToolkitLimits {
                tools: 0,
                ..default
            },
            McpToolkitLimits {
                discovery_pages: 0,
                ..default
            },
            McpToolkitLimits {
                discovery_bytes: 0,
                ..default
            },
            McpToolkitLimits {
                cursor_bytes: 0,
                ..default
            },
            McpToolkitLimits {
                schema_bytes: 0,
                ..default
            },
            McpToolkitLimits {
                schema_depth: 0,
                ..default
            },
            McpToolkitLimits {
                schema_nodes: 0,
                ..default
            },
            McpToolkitLimits {
                argument_bytes: 0,
                ..default
            },
            McpToolkitLimits {
                result_bytes: 0,
                ..default
            },
            McpToolkitLimits {
                discovery_timeout: Duration::ZERO,
                ..default
            },
        ] {
            assert!(invalid.validate().is_err());
        }
        for invalid in [
            McpToolkitLimits {
                tools: MAX_TOOLS + 1,
                ..default
            },
            McpToolkitLimits {
                discovery_pages: MAX_DISCOVERY_PAGES + 1,
                ..default
            },
            McpToolkitLimits {
                discovery_bytes: MAX_DISCOVERY_BYTES + 1,
                ..default
            },
            McpToolkitLimits {
                cursor_bytes: MAX_CURSOR_BYTES + 1,
                ..default
            },
            McpToolkitLimits {
                schema_bytes: MAX_SCHEMA_BYTES + 1,
                ..default
            },
            McpToolkitLimits {
                schema_depth: MAX_SCHEMA_DEPTH + 1,
                ..default
            },
            McpToolkitLimits {
                schema_nodes: MAX_SCHEMA_NODES + 1,
                ..default
            },
            McpToolkitLimits {
                argument_bytes: MAX_ARGUMENT_BYTES + 1,
                ..default
            },
            McpToolkitLimits {
                result_bytes: MAX_RESULT_BYTES + 1,
                ..default
            },
            McpToolkitLimits {
                discovery_timeout: MAX_DISCOVERY_TIMEOUT + Duration::from_nanos(1),
                ..default
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn stdio_limits_reject_zero_and_over_ceiling_fields() {
        let default = McpStdioLimits::default();
        assert!(default.validate().is_ok());
        for invalid in [
            McpStdioLimits {
                inbound_frame_bytes: 0,
                ..default
            },
            McpStdioLimits {
                outbound_frame_bytes: 0,
                ..default
            },
            McpStdioLimits {
                max_in_flight: 0,
                ..default
            },
            McpStdioLimits {
                cancelled_request_ids: 0,
                ..default
            },
            McpStdioLimits {
                request_timeout: Duration::ZERO,
                ..default
            },
            McpStdioLimits {
                close_timeout: Duration::ZERO,
                ..default
            },
            McpStdioLimits {
                inbound_frame_bytes: MAX_FRAME_BYTES + 1,
                ..default
            },
            McpStdioLimits {
                outbound_frame_bytes: MAX_FRAME_BYTES + 1,
                ..default
            },
            McpStdioLimits {
                max_in_flight: MAX_IN_FLIGHT + 1,
                ..default
            },
            McpStdioLimits {
                cancelled_request_ids: MAX_CANCELLED_REQUEST_IDS + 1,
                ..default
            },
            McpStdioLimits {
                request_timeout: MAX_REQUEST_TIMEOUT + Duration::from_nanos(1),
                ..default
            },
            McpStdioLimits {
                close_timeout: MAX_CLOSE_TIMEOUT + Duration::from_nanos(1),
                ..default
            },
        ] {
            assert!(invalid.validate().is_err());
        }
    }

    #[test]
    fn invalid_limits_identify_the_rejected_field() {
        let error = McpToolkitLimits {
            schema_depth: 0,
            ..McpToolkitLimits::default()
        }
        .validate();
        assert_eq!(
            error,
            Err(McpClientError::InvalidLimits {
                field: "schema_depth"
            })
        );
    }
}
