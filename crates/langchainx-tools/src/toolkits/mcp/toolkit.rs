use std::{collections::HashSet, sync::Arc};

use crate::{DynTool, toolkits::common::normalize_unique_tool_names};

use super::{
    McpClient, McpClientError, McpToolDefinition, McpToolkitError, McpToolkitLimits,
    framing::serialized_size_bounded, tool::McpTool,
};

/// A validated, bounded collection of tools discovered from one MCP client.
#[derive(Clone)]
pub struct McpToolkit {
    client: Arc<dyn McpClient>,
    definitions: Vec<McpToolDefinition>,
    exposed_names: Vec<String>,
    limits: McpToolkitLimits,
}

impl McpToolkit {
    /// Discovers and validates all tools exposed by `client` within `limits`.
    ///
    /// # Errors
    ///
    /// Returns [`McpToolkitError`] when limits are invalid, discovery fails or times out,
    /// pagination bounds are exceeded, or a discovered name or schema is unsafe.
    pub async fn new(
        client: Arc<dyn McpClient>,
        limits: McpToolkitLimits,
    ) -> Result<Self, McpToolkitError> {
        limits.validate()?;
        let definitions = tokio::time::timeout(
            limits.discovery_timeout,
            discover_tools(client.as_ref(), limits),
        )
        .await
        .map_err(|_| McpClientError::Timeout)??;
        let exposed_names =
            normalize_unique_tool_names(definitions.iter().map(|tool| &tool.name), None)
                .map_err(map_name_error)?;
        Ok(Self {
            client,
            definitions,
            exposed_names,
            limits,
        })
    }

    /// Applies a normalized prefix to every exposed tool name.
    ///
    /// # Errors
    ///
    /// Returns [`McpToolkitError`] when the prefix cannot produce valid unique tool names.
    pub fn with_name_prefix(mut self, prefix: impl Into<String>) -> Result<Self, McpToolkitError> {
        let prefix = prefix.into();
        self.exposed_names = normalize_unique_tool_names(
            self.definitions.iter().map(|tool| &tool.name),
            Some(&prefix),
        )
        .map_err(map_name_error)?;
        Ok(self)
    }

    /// Returns generated adapters for all discovered tools.
    pub fn tools(&self) -> Vec<Arc<dyn DynTool>> {
        self.definitions
            .iter()
            .cloned()
            .zip(self.exposed_names.iter().cloned())
            .map(|(definition, exposed_name)| {
                Arc::new(McpTool::new(
                    self.client.clone(),
                    definition,
                    exposed_name,
                    self.limits,
                )) as Arc<dyn DynTool>
            })
            .collect()
    }

    /// Delegates coordinated idempotent closure to the underlying client.
    ///
    /// # Errors
    ///
    /// Returns [`McpClientError`] when the underlying client cannot complete closure.
    pub async fn close(&self) -> Result<(), McpClientError> {
        self.client.close().await
    }
}

async fn discover_tools(
    client: &dyn McpClient,
    limits: McpToolkitLimits,
) -> Result<Vec<McpToolDefinition>, McpToolkitError> {
    let mut definitions = Vec::new();
    let mut cursor = None;
    let mut seen_cursors = HashSet::new();
    let mut discovery_bytes = 0usize;

    for _ in 0..limits.discovery_pages {
        let page = client.list_tools(cursor.take()).await?;
        let remaining = limits.discovery_bytes.saturating_sub(discovery_bytes);
        let page_bytes =
            serialized_size_bounded(&page, remaining).map_err(|error| match error {
                McpClientError::MessageTooLarge { .. } => McpClientError::MessageTooLarge {
                    limit: limits.discovery_bytes,
                },
                _ => McpClientError::Protocol("invalid_tool_page".into()),
            })?;
        discovery_bytes = discovery_bytes.saturating_add(page_bytes);
        if discovery_bytes > limits.discovery_bytes {
            return Err(McpClientError::MessageTooLarge {
                limit: limits.discovery_bytes,
            }
            .into());
        }
        if definitions.len().saturating_add(page.tools.len()) > limits.tools {
            return Err(McpToolkitError::TooManyTools {
                limit: limits.tools,
            });
        }
        for definition in &page.tools {
            validate_definition(definition, limits)?;
        }
        for mut definition in page.tools {
            let mut budget = SchemaBudget::new(limits.schema_nodes);
            sanitize_schema(
                &mut definition.input_schema,
                &mut budget,
                1,
                limits.schema_depth,
            )?;
            definitions.push(definition);
        }

        let Some(next_cursor) = page.next_cursor else {
            return Ok(definitions);
        };
        if next_cursor.len() > limits.cursor_bytes {
            return Err(McpClientError::MessageTooLarge {
                limit: limits.cursor_bytes,
            }
            .into());
        }
        if next_cursor.is_empty() || !seen_cursors.insert(next_cursor.clone()) {
            return Err(McpToolkitError::CursorLoop);
        }
        cursor = Some(next_cursor);
    }

    Err(McpClientError::Protocol("discovery_page_limit".into()).into())
}

fn validate_definition(
    definition: &McpToolDefinition,
    limits: McpToolkitLimits,
) -> Result<(), McpToolkitError> {
    serialized_size_bounded(&definition.input_schema, limits.schema_bytes).map_err(|error| {
        match error {
            McpClientError::MessageTooLarge { .. } => {
                McpToolkitError::InvalidToolSchema("schema_too_large".into())
            }
            _ => McpToolkitError::InvalidToolSchema("serialization_failed".into()),
        }
    })?;
    if definition
        .input_schema
        .get("type")
        .and_then(serde_json::Value::as_str)
        != Some("object")
        || !definition
            .input_schema
            .get("properties")
            .is_some_and(serde_json::Value::is_object)
    {
        return Err(McpToolkitError::InvalidToolSchema(
            "object_schema_required".into(),
        ));
    }
    Ok(())
}

struct SchemaBudget {
    remaining_nodes: usize,
}

impl SchemaBudget {
    fn new(nodes: usize) -> Self {
        Self {
            remaining_nodes: nodes,
        }
    }

    fn visit(&mut self, depth: usize, max_depth: usize) -> Result<(), McpToolkitError> {
        if depth > max_depth {
            return Err(invalid_schema("schema_depth_exceeded"));
        }
        self.remaining_nodes = self
            .remaining_nodes
            .checked_sub(1)
            .ok_or_else(|| invalid_schema("schema_nodes_exceeded"))?;
        Ok(())
    }
}

fn sanitize_schema(
    value: &mut serde_json::Value,
    budget: &mut SchemaBudget,
    depth: usize,
    max_depth: usize,
) -> Result<(), McpToolkitError> {
    budget.visit(depth, max_depth)?;
    match value {
        serde_json::Value::Object(object) => {
            sanitize_schema_object(object, budget, depth, max_depth)?
        }
        serde_json::Value::Bool(_) => {}
        _ => return Err(invalid_schema("schema_node_must_be_object_or_boolean")),
    }
    Ok(())
}

fn sanitize_schema_object(
    object: &mut serde_json::Map<String, serde_json::Value>,
    budget: &mut SchemaBudget,
    depth: usize,
    max_depth: usize,
) -> Result<(), McpToolkitError> {
    let source = std::mem::take(object);
    for (key, mut value) in source {
        match key.as_str() {
            "properties" | "patternProperties" | "dependentSchemas" => {
                sanitize_named_schemas(&mut value, budget, depth, max_depth)?
            }
            "items"
            | "contains"
            | "additionalProperties"
            | "unevaluatedProperties"
            | "propertyNames"
            | "not"
            | "if"
            | "then"
            | "else" => {
                sanitize_schema(&mut value, budget, depth + 1, max_depth)?;
            }
            "allOf" | "anyOf" | "oneOf" | "prefixItems" => {
                let schemas = value
                    .as_array_mut()
                    .ok_or_else(|| invalid_schema("schema_array_required"))?;
                for schema in schemas {
                    sanitize_schema(schema, budget, depth + 1, max_depth)?;
                }
            }
            "required" => validate_string_array(&value, "invalid_required")?,
            "dependentRequired" => validate_dependent_required(&value)?,
            "$ref" | "$dynamicRef" | "$defs" | "definitions" => {
                return Err(invalid_schema("schema_references_unsupported"));
            }
            "type" => validate_type_keyword(&value)?,
            "enum" => {
                if value.as_array().is_none_or(Vec::is_empty) {
                    return Err(invalid_schema("invalid_enum"));
                }
            }
            "const" => {}
            "multipleOf" => {
                if value.as_f64().is_none_or(|number| number <= 0.0) {
                    return Err(invalid_schema("invalid_multiple_of"));
                }
            }
            "maximum" | "exclusiveMaximum" | "minimum" | "exclusiveMinimum" => {
                if !value.is_number() {
                    return Err(invalid_schema("numeric_bound_required"));
                }
            }
            "maxLength" | "minLength" | "maxItems" | "minItems" | "maxContains" | "minContains"
            | "maxProperties" | "minProperties" => {
                if value.as_u64().is_none() {
                    return Err(invalid_schema("nonnegative_integer_required"));
                }
            }
            "pattern" => {
                if !value.is_string() {
                    return Err(invalid_schema("string_pattern_required"));
                }
            }
            "uniqueItems" => {
                if !value.is_boolean() {
                    return Err(invalid_schema("boolean_validation_required"));
                }
            }
            "description" | "title" | "default" | "examples" | "example" | "$comment"
            | "deprecated" | "readOnly" | "writeOnly" => continue,
            _ if key.to_ascii_lowercase().starts_with("x-") => continue,
            _ => return Err(invalid_schema("unsupported_schema_keyword")),
        }
        object.insert(key, value);
    }

    if let Some(required) = object.get("required") {
        let properties = object
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| invalid_schema("required_without_properties"))?;
        let names = required
            .as_array()
            .ok_or_else(|| invalid_schema("invalid_required"))?;
        if names.iter().any(|name| {
            name.as_str()
                .is_none_or(|name| !properties.contains_key(name))
        }) {
            return Err(invalid_schema("required_property_missing"));
        }
    }
    Ok(())
}

fn sanitize_named_schemas(
    value: &mut serde_json::Value,
    budget: &mut SchemaBudget,
    depth: usize,
    max_depth: usize,
) -> Result<(), McpToolkitError> {
    let schemas = value
        .as_object_mut()
        .ok_or_else(|| invalid_schema("named_schema_map_required"))?;
    for schema in schemas.values_mut() {
        sanitize_schema(schema, budget, depth + 1, max_depth)?;
    }
    Ok(())
}

fn validate_string_array(value: &serde_json::Value, code: &str) -> Result<(), McpToolkitError> {
    let values = value.as_array().ok_or_else(|| invalid_schema(code))?;
    let mut seen = HashSet::new();
    if values.iter().any(|value| {
        value
            .as_str()
            .is_none_or(|value| value.is_empty() || !seen.insert(value))
    }) {
        return Err(invalid_schema(code));
    }
    Ok(())
}

fn validate_dependent_required(value: &serde_json::Value) -> Result<(), McpToolkitError> {
    let dependencies = value
        .as_object()
        .ok_or_else(|| invalid_schema("invalid_dependent_required"))?;
    for required in dependencies.values() {
        validate_string_array(required, "invalid_dependent_required")?;
    }
    Ok(())
}

fn validate_type_keyword(value: &serde_json::Value) -> Result<(), McpToolkitError> {
    const TYPES: &[&str] = &[
        "null", "boolean", "object", "array", "number", "integer", "string",
    ];
    let valid = value.as_str().is_some_and(|kind| TYPES.contains(&kind))
        || value.as_array().is_some_and(|kinds| {
            let mut seen = HashSet::new();
            !kinds.is_empty()
                && kinds.iter().all(|kind| {
                    kind.as_str()
                        .is_some_and(|kind| TYPES.contains(&kind) && seen.insert(kind))
                })
        });
    if valid {
        Ok(())
    } else {
        Err(invalid_schema("invalid_type"))
    }
}

fn invalid_schema(code: &str) -> McpToolkitError {
    McpToolkitError::InvalidToolSchema(code.into())
}

fn map_name_error(error: crate::toolkits::common::ToolNameError) -> McpToolkitError {
    match error {
        crate::toolkits::common::ToolNameError::Duplicate(name) => {
            McpToolkitError::DuplicateToolName(name)
        }
        _ => McpToolkitError::InvalidToolSchema("invalid_tool_name".into()),
    }
}
