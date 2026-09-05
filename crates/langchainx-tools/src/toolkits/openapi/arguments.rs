use std::borrow::Cow;

use http::{HeaderMap, HeaderName, HeaderValue};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Map, Value};
use url::Url;

use super::{
    AuthorizedOrigin, HttpLimits, HttpOrigin, HttpRequest, OpenApiToolkitError,
    operation::{OpenApiOperation, OpenApiParameter, ParameterLocation},
};

pub(crate) struct PreparedRequest {
    method: String,
    url: Url,
    headers: HeaderMap<SecretString>,
    body: Option<Vec<u8>>,
}

impl PreparedRequest {
    pub(crate) fn url(&self) -> &Url {
        &self.url
    }

    pub(crate) fn authorize(
        self,
        authorized_origin: AuthorizedOrigin,
    ) -> Result<HttpRequest, OpenApiToolkitError> {
        if HttpOrigin::try_from_url(&self.url)? != *authorized_origin.origin() {
            return Err(OpenApiToolkitError::EgressDenied(
                "authorized origin does not match final request origin".into(),
            ));
        }
        Ok(HttpRequest::new(
            self.method,
            self.url,
            self.headers,
            self.body,
            authorized_origin,
        ))
    }

    #[cfg(feature = "fuzzing")]
    pub(crate) fn fuzz_sizes(&self) -> (usize, usize) {
        let header_bytes = self
            .headers
            .iter()
            .map(|(name, value)| name.as_str().len() + value.expose_secret().len())
            .sum();
        (header_bytes, self.body.as_deref().map_or(0, <[u8]>::len))
    }
}

pub(crate) fn prepare_request(
    operation: &OpenApiOperation,
    input: &Value,
    credential_headers: &HeaderMap<SecretString>,
    limits: HttpLimits,
) -> Result<PreparedRequest, OpenApiToolkitError> {
    bounded_json_len(input, limits.request_body_bytes)?;
    validate_value(input, &operation.tool_schema)?;
    let input = input
        .as_object()
        .ok_or_else(|| invalid_arguments("root input must be an object"))?;
    reject_unknown(input, &["path", "query", "headers", "cookies", "body"])?;
    let mut url = operation.server.clone();
    url.set_query(None);
    url.set_fragment(None);
    let mut path = operation.path.clone();
    let mut headers = HeaderMap::<SecretString>::with_capacity(
        credential_headers.len() + operation.parameters.len() + 2,
    );
    for (name, value) in credential_headers {
        validate_header_value(value.expose_secret())?;
        headers.insert(name.clone(), value.clone());
    }
    let mut query_parts = Vec::new();
    let mut cookie_parts = Vec::new();
    {
        let mut serialization = SerializationState {
            path: &mut path,
            query_parts: &mut query_parts,
            headers: &mut headers,
            cookies: &mut cookie_parts,
            credentials: credential_headers,
            limits,
        };
        for parameter in &operation.parameters {
            let group_name = location_group(&parameter.location);
            let group = input.get(group_name).map(as_object).transpose()?;
            if let Some(group) = group {
                let allowed: Vec<_> = operation
                    .parameters
                    .iter()
                    .filter(|candidate| candidate.location == parameter.location)
                    .map(|candidate| candidate.name.as_str())
                    .collect();
                reject_unknown(group, &allowed)?;
            }
            let value = group.and_then(|group| group.get(&parameter.name));
            if value.is_none() && parameter.required {
                return Err(invalid_arguments("a required parameter is missing"));
            }
            let Some(value) = value else { continue };
            validate_value(value, &parameter.schema)?;
            serialize_parameter(parameter, value, &mut serialization)?;
        }
    }
    ensure_required_groups_known(operation, input)?;
    if path.contains('{') || path.contains('}') {
        return Err(invalid_arguments("a path parameter is missing"));
    }
    let base = operation.server.path().trim_end_matches('/');
    let final_path = format!("{base}{path}");
    if !final_path.starts_with('/') || final_path.starts_with("//") {
        return Err(invalid_arguments("path could alter URL authority"));
    }
    url.set_path(&final_path);
    if !query_parts.is_empty() {
        url.set_query(Some(&query_parts.join("&")));
    }
    if !cookie_parts.is_empty() {
        insert_header(
            &mut headers,
            HeaderName::from_static("cookie"),
            cookie_parts.join("; "),
            credential_headers,
        )?;
    }
    let body = match input.get("body") {
        Some(body) if operation.body_schema.is_some() => {
            if let Some(schema) = operation.body_schema.as_ref() {
                validate_value(body, schema)?;
            }
            let serialized = serde_json::to_vec(body)
                .map_err(|_| invalid_arguments("request body cannot be serialized"))?;
            if serialized.len() > limits.request_body_bytes {
                return Err(OpenApiToolkitError::RequestTooLarge {
                    limit: limits.request_body_bytes,
                });
            }
            insert_header(
                &mut headers,
                HeaderName::from_static("content-type"),
                "application/json".into(),
                credential_headers,
            )?;
            Some(serialized)
        }
        Some(_) => return Err(invalid_arguments("operation does not accept a body")),
        None if operation.body_required => {
            return Err(invalid_arguments("request body is required"));
        }
        None => None,
    };
    if url.as_str().len() > limits.url_bytes {
        return Err(OpenApiToolkitError::RequestTooLarge {
            limit: limits.url_bytes,
        });
    }
    let header_bytes = headers
        .iter()
        .map(|(name, value)| name.as_str().len() + value.expose_secret().len())
        .sum::<usize>();
    if header_bytes > limits.header_bytes {
        return Err(OpenApiToolkitError::RequestTooLarge {
            limit: limits.header_bytes,
        });
    }
    Ok(PreparedRequest {
        method: operation.method.clone(),
        url,
        headers,
        body,
    })
}

pub(crate) fn is_protected_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "upgrade"
            | "authorization"
            | "www-authenticate"
            | "set-cookie"
            | "x-api-key"
            | "content-type"
    )
}

pub(crate) fn is_forbidden_credential_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "content-length"
            | "transfer-encoding"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "upgrade"
            | "www-authenticate"
            | "set-cookie"
            | "content-type"
    )
}

struct SerializationState<'a> {
    path: &'a mut String,
    query_parts: &'a mut Vec<String>,
    headers: &'a mut HeaderMap<SecretString>,
    cookies: &'a mut Vec<String>,
    credentials: &'a HeaderMap<SecretString>,
    limits: HttpLimits,
}

fn serialize_parameter(
    parameter: &OpenApiParameter,
    value: &Value,
    state: &mut SerializationState<'_>,
) -> Result<(), OpenApiToolkitError> {
    let item_limit = match parameter.location {
        ParameterLocation::Path | ParameterLocation::Query => state.limits.url_bytes,
        ParameterLocation::Header | ParameterLocation::Cookie => state.limits.header_bytes,
    }
    .min(1_024);
    let values = values(value, item_limit)?;
    match parameter.location {
        ParameterLocation::Path => {
            if values
                .iter()
                .any(|value| matches!(value.as_ref(), "." | ".."))
            {
                return Err(invalid_arguments("path parameter contains a dot segment"));
            }
            let serialized = values
                .iter()
                .map(|value| encode_checked(value, state.limits.url_bytes))
                .collect::<Result<Vec<_>, _>>()?
                .join(",");
            let placeholder = format!("{{{}}}", parameter.name);
            if !state.path.contains(&placeholder) {
                return Err(invalid_arguments(
                    "path parameter has no template placeholder",
                ));
            }
            *state.path = state.path.replace(&placeholder, &serialized);
        }
        ParameterLocation::Query => {
            if value.is_array() && parameter.explode {
                let name = encode_checked(&parameter.name, state.limits.url_bytes)?;
                for value in &values {
                    let value = encode_checked(value, state.limits.url_bytes)?;
                    push_bounded(
                        state.query_parts,
                        format!("{name}={value}"),
                        state.limits.url_bytes,
                    )?;
                }
            } else {
                let name = encode_checked(&parameter.name, state.limits.url_bytes)?;
                let value = values
                    .iter()
                    .map(|value| encode_checked(value, state.limits.url_bytes))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(",");
                push_bounded(
                    state.query_parts,
                    format!("{name}={value}"),
                    state.limits.url_bytes,
                )?;
            }
        }
        ParameterLocation::Header => {
            let name = HeaderName::from_bytes(parameter.name.as_bytes())
                .map_err(|_| invalid_arguments("invalid header name"))?;
            if is_protected_header(&name) || name == http::header::COOKIE {
                return Err(OpenApiToolkitError::ProtectedHeader(name.to_string()));
            }
            let value = values.join(",");
            if value.len() > state.limits.header_bytes {
                return Err(OpenApiToolkitError::RequestTooLarge {
                    limit: state.limits.header_bytes,
                });
            }
            insert_header(state.headers, name, value, state.credentials)?;
        }
        ParameterLocation::Cookie => {
            let name = encode_checked(&parameter.name, state.limits.header_bytes)?;
            let separator = if value.is_array() { "&" } else { "" };
            let serialized = values
                .iter()
                .map(|value| {
                    encode_checked(value, state.limits.header_bytes)
                        .map(|value| format!("{name}={value}"))
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(separator);
            push_bounded(state.cookies, serialized, state.limits.header_bytes)?;
        }
    }
    Ok(())
}

fn insert_header(
    headers: &mut HeaderMap<SecretString>,
    name: HeaderName,
    value: String,
    credentials: &HeaderMap<SecretString>,
) -> Result<(), OpenApiToolkitError> {
    if credentials.contains_key(&name) || headers.contains_key(&name) {
        return Err(OpenApiToolkitError::ProtectedHeader(name.to_string()));
    }
    validate_header_value(&value)?;
    headers.insert(name, SecretString::from(value));
    Ok(())
}

fn validate_header_value(value: &str) -> Result<(), OpenApiToolkitError> {
    HeaderValue::from_str(value)
        .map(|_| ())
        .map_err(|_| invalid_arguments("invalid header value"))
}

fn values(value: &Value, item_limit: usize) -> Result<Vec<Cow<'_, str>>, OpenApiToolkitError> {
    match value {
        Value::Array(values) if values.len() <= item_limit => values.iter().map(scalar).collect(),
        Value::Array(_) => Err(OpenApiToolkitError::RequestTooLarge { limit: item_limit }),
        _ => Ok(vec![scalar(value)?]),
    }
}

fn scalar(value: &Value) -> Result<Cow<'_, str>, OpenApiToolkitError> {
    match value {
        Value::String(value) => Ok(Cow::Borrowed(value)),
        Value::Bool(true) => Ok(Cow::Borrowed("true")),
        Value::Bool(false) => Ok(Cow::Borrowed("false")),
        Value::Number(value) => Ok(Cow::Owned(value.to_string())),
        _ => Err(invalid_arguments("parameter values must be primitive")),
    }
}

fn validate_value(value: &Value, schema: &Value) -> Result<(), OpenApiToolkitError> {
    if value.is_null() && schema.get("nullable").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    if schema
        .get("enum")
        .and_then(Value::as_array)
        .is_some_and(|allowed| !allowed.contains(value))
    {
        return Err(invalid_arguments("value is outside the schema enum"));
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("string") => validate_string(value, schema),
        Some("boolean") if value.is_boolean() => Ok(()),
        Some("integer") if value.as_i64().is_some() || value.as_u64().is_some() => {
            validate_integer(value, schema)
        }
        Some("number") if value.is_number() => Ok(()),
        Some("array") => validate_array(value, schema),
        Some("object") => validate_object(value, schema),
        None => Ok(()),
        _ => Err(invalid_arguments("value does not match its schema")),
    }
}

fn validate_string(value: &Value, schema: &Value) -> Result<(), OpenApiToolkitError> {
    let value = value
        .as_str()
        .ok_or_else(|| invalid_arguments("value must be a string"))?;
    let length = value.chars().count() as u64;
    if schema
        .get("minLength")
        .and_then(Value::as_u64)
        .is_some_and(|min| length < min)
        || schema
            .get("maxLength")
            .and_then(Value::as_u64)
            .is_some_and(|max| length > max)
    {
        return Err(invalid_arguments("string length violates schema"));
    }
    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        let pattern = regex::Regex::new(pattern)
            .map_err(|_| invalid_arguments("schema pattern is invalid"))?;
        if !pattern.is_match(value) {
            return Err(invalid_arguments("string does not match schema pattern"));
        }
    }
    Ok(())
}

fn validate_integer(value: &Value, schema: &Value) -> Result<(), OpenApiToolkitError> {
    let value =
        exact_integer(value).ok_or_else(|| invalid_arguments("value must be an integer"))?;
    if schema
        .get("minimum")
        .and_then(exact_integer)
        .is_some_and(|min| value < min)
        || schema
            .get("maximum")
            .and_then(exact_integer)
            .is_some_and(|max| value > max)
    {
        return Err(invalid_arguments("number violates schema range"));
    }
    if schema.get("exclusiveMinimum").and_then(Value::as_bool) == Some(true)
        && schema
            .get("minimum")
            .and_then(exact_integer)
            .is_some_and(|minimum| value <= minimum)
        || schema.get("exclusiveMaximum").and_then(Value::as_bool) == Some(true)
            && schema
                .get("maximum")
                .and_then(exact_integer)
                .is_some_and(|maximum| value >= maximum)
    {
        return Err(invalid_arguments("number violates exclusive schema range"));
    }
    Ok(())
}

fn exact_integer(value: &Value) -> Option<i128> {
    value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
}

fn validate_array(value: &Value, schema: &Value) -> Result<(), OpenApiToolkitError> {
    let values = value
        .as_array()
        .ok_or_else(|| invalid_arguments("value must be an array"))?;
    let length = values.len() as u64;
    if schema
        .get("minItems")
        .and_then(Value::as_u64)
        .is_some_and(|min| length < min)
        || schema
            .get("maxItems")
            .and_then(Value::as_u64)
            .is_some_and(|max| length > max)
    {
        return Err(invalid_arguments("array length violates schema"));
    }
    if schema.get("uniqueItems").and_then(Value::as_bool) == Some(true) && values.len() > 1_024 {
        return Err(invalid_arguments("unique array exceeds validation bound"));
    }
    if schema.get("uniqueItems").and_then(Value::as_bool) == Some(true)
        && values
            .iter()
            .enumerate()
            .any(|(index, value)| values[..index].contains(value))
    {
        return Err(invalid_arguments("array items must be unique"));
    }
    if let Some(items) = schema.get("items") {
        for value in values {
            validate_value(value, items)?;
        }
    }
    Ok(())
}

fn validate_object(value: &Value, schema: &Value) -> Result<(), OpenApiToolkitError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid_arguments("value must be an object"))?;
    let length = object.len() as u64;
    if schema
        .get("minProperties")
        .and_then(Value::as_u64)
        .is_some_and(|minimum| length < minimum)
        || schema
            .get("maxProperties")
            .and_then(Value::as_u64)
            .is_some_and(|maximum| length > maximum)
    {
        return Err(invalid_arguments("object size violates schema"));
    }
    if let Some(required) = schema.get("required").and_then(Value::as_array)
        && required
            .iter()
            .filter_map(Value::as_str)
            .any(|name| !object.contains_key(name))
    {
        return Err(invalid_arguments("required object property is missing"));
    }
    let properties = schema.get("properties").and_then(Value::as_object);
    for (name, value) in object {
        if let Some(property) = properties.and_then(|properties| properties.get(name)) {
            validate_value(value, property)?;
            continue;
        }
        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) => {
                return Err(invalid_arguments("additional object property is forbidden"));
            }
            Some(additional) if additional.is_object() => validate_value(value, additional)?,
            _ => {}
        }
    }
    Ok(())
}

fn as_object(value: &Value) -> Result<&Map<String, Value>, OpenApiToolkitError> {
    value
        .as_object()
        .ok_or_else(|| invalid_arguments("argument group must be an object"))
}

fn reject_unknown(
    object: &Map<String, Value>,
    allowed: &[&str],
) -> Result<(), OpenApiToolkitError> {
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(invalid_arguments("input contains an unknown field"));
    }
    Ok(())
}

fn ensure_required_groups_known(
    operation: &OpenApiOperation,
    input: &Map<String, Value>,
) -> Result<(), OpenApiToolkitError> {
    for group in ["path", "query", "headers", "cookies"] {
        if let Some(value) = input.get(group) {
            let object = as_object(value)?;
            let allowed: Vec<_> = operation
                .parameters
                .iter()
                .filter(|parameter| location_group(&parameter.location) == group)
                .map(|parameter| parameter.name.as_str())
                .collect();
            reject_unknown(object, &allowed)?;
        }
    }
    Ok(())
}

fn location_group(location: &ParameterLocation) -> &'static str {
    match location {
        ParameterLocation::Path => "path",
        ParameterLocation::Query => "query",
        ParameterLocation::Header => "headers",
        ParameterLocation::Cookie => "cookies",
    }
}

fn encode_checked(value: &str, limit: usize) -> Result<String, OpenApiToolkitError> {
    let encoded_len = value.bytes().try_fold(0usize, |length, byte| {
        let width = if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            1
        } else {
            3
        };
        length.checked_add(width)
    });
    if encoded_len.is_none_or(|size| size > limit) {
        return Err(OpenApiToolkitError::RequestTooLarge { limit });
    }
    Ok(urlencoding::encode(value).into_owned())
}

fn push_bounded(
    parts: &mut Vec<String>,
    part: String,
    limit: usize,
) -> Result<(), OpenApiToolkitError> {
    let current = parts.iter().map(String::len).sum::<usize>();
    if current
        .checked_add(parts.len())
        .and_then(|size| size.checked_add(part.len()))
        .is_none_or(|size| size > limit)
    {
        return Err(OpenApiToolkitError::RequestTooLarge { limit });
    }
    parts.push(part);
    Ok(())
}

fn bounded_json_len(value: &Value, limit: usize) -> Result<usize, OpenApiToolkitError> {
    fn add(total: &mut usize, amount: usize, limit: usize) -> Result<(), OpenApiToolkitError> {
        *total = total
            .checked_add(amount)
            .filter(|total| *total <= limit)
            .ok_or(OpenApiToolkitError::RequestTooLarge { limit })?;
        Ok(())
    }
    fn string_len(value: &str) -> usize {
        2 + value
            .chars()
            .map(|character| match character {
                '"' | '\\' | '\u{0008}' | '\u{000c}' | '\n' | '\r' | '\t' => 2,
                '\u{0000}'..='\u{001f}' => 6,
                character => character.len_utf8(),
            })
            .sum::<usize>()
    }
    fn visit(
        value: &Value,
        total: &mut usize,
        nodes: &mut usize,
        limit: usize,
        depth: usize,
    ) -> Result<(), OpenApiToolkitError> {
        *nodes = nodes.saturating_add(1);
        if depth > 64 || *nodes > 100_000 {
            return Err(OpenApiToolkitError::RequestTooLarge { limit });
        }
        match value {
            Value::Null => add(total, 4, limit),
            Value::Bool(true) => add(total, 4, limit),
            Value::Bool(false) => add(total, 5, limit),
            Value::Number(number) => add(total, number.to_string().len(), limit),
            Value::String(value) => add(total, string_len(value), limit),
            Value::Array(values) => {
                add(total, 2 + values.len().saturating_sub(1), limit)?;
                for value in values {
                    visit(value, total, nodes, limit, depth + 1)?;
                }
                Ok(())
            }
            Value::Object(object) => {
                add(total, 2 + object.len().saturating_sub(1), limit)?;
                for (name, value) in object {
                    add(total, string_len(name) + 1, limit)?;
                    visit(value, total, nodes, limit, depth + 1)?;
                }
                Ok(())
            }
        }
    }
    let mut total = 0;
    let mut nodes = 0;
    visit(value, &mut total, &mut nodes, limit, 0)?;
    Ok(total)
}

fn invalid_arguments(message: &str) -> OpenApiToolkitError {
    OpenApiToolkitError::InvalidArguments(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolkits::openapi::{OpenApiFormat, OpenApiLimits, spec::parse_operations};
    use std::net::{IpAddr, Ipv4Addr};

    fn operation() -> OpenApiOperation {
        let specification = r#"{"openapi":"3.0.0","servers":[{"url":"https://api.example/root"}],"paths":{"/items/{id}":{"post":{"operationId":"createItem","parameters":[{"name":"id","in":"path","required":true,"style":"simple","schema":{"type":"string"}},{"name":"tag","in":"query","style":"form","explode":true,"schema":{"type":"array","items":{"type":"string"}}},{"name":"x-mode","in":"header","style":"simple","schema":{"type":"string"}},{"name":"session","in":"cookie","style":"form","explode":true,"schema":{"type":"string"}}],"requestBody":{"required":true,"content":{"application/json":{"schema":{"type":"object"}}}},"responses":{}}}}}"#;
        parse_operations(
            specification,
            OpenApiFormat::Json,
            None,
            None,
            OpenApiLimits::default(),
        )
        .unwrap()
        .remove(0)
    }

    #[test]
    fn maps_nested_arguments_and_supported_styles() {
        let operation = operation();
        let input = serde_json::json!({"path":{"id":"a/b"},"query":{"tag":["red","blue"]},"headers":{"x-mode":"fast"},"cookies":{"session":"abc"},"body":{"name":"widget"}});
        let prepared = prepare_request(
            &operation,
            &input,
            &HeaderMap::with_capacity(0),
            HttpLimits::default(),
        )
        .unwrap();
        let origin = HttpOrigin::try_from_url(&operation.server).unwrap();
        let authorized =
            AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))]).unwrap();
        let request = prepared.authorize(authorized).unwrap();
        assert_eq!(request.url().path(), "/root/items/a%2Fb");
        assert_eq!(request.url().query(), Some("tag=red&tag=blue"));
        assert_eq!(request.body(), Some(br#"{"name":"widget"}"#.as_slice()));
        assert!(request.headers().contains_key("x-mode"));
        assert!(request.headers().contains_key("cookie"));
        assert!(request.headers().contains_key("content-type"));
    }

    #[test]
    fn rejects_missing_unknown_protected_and_oversized_arguments() {
        let operation = operation();
        let credentials = HeaderMap::with_capacity(0);
        for input in [
            serde_json::json!({"body":{}}),
            serde_json::json!({"path":{"id":"x","unknown":1},"body":{}}),
            serde_json::json!({"path":{"id":"x"},"unknown":{},"body":{}}),
        ] {
            assert!(
                prepare_request(&operation, &input, &credentials, HttpLimits::default()).is_err()
            );
        }
        let limits = HttpLimits {
            request_body_bytes: 2,
            ..HttpLimits::default()
        };
        assert!(
            prepare_request(
                &operation,
                &serde_json::json!({"path":{"id":"x"},"body":{"large":true}}),
                &credentials,
                limits
            )
            .is_err()
        );
    }

    #[test]
    fn authorization_requires_exact_final_origin() {
        let operation = operation();
        let prepared = prepare_request(
            &operation,
            &serde_json::json!({"path":{"id":"x"},"body":{}}),
            &HeaderMap::with_capacity(0),
            HttpLimits::default(),
        )
        .unwrap();
        let wrong =
            HttpOrigin::try_from_url(&Url::parse("https://other.example").unwrap()).unwrap();
        let authorized =
            AuthorizedOrigin::new(wrong, vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))]).unwrap();
        assert!(prepared.authorize(authorized).is_err());
    }

    #[test]
    fn identifies_transport_and_credential_headers_as_protected() {
        for name in [
            "host",
            "content-length",
            "transfer-encoding",
            "connection",
            "authorization",
            "proxy-authorization",
            "x-api-key",
        ] {
            assert!(
                is_protected_header(&HeaderName::from_static(name)),
                "{name}"
            );
        }
    }

    #[test]
    fn cookie_array_uses_form_explode_encoding() {
        let specification = r#"{"openapi":"3.0.0","servers":[{"url":"https://api.example"}],"paths":{"/x":{"get":{"operationId":"x","parameters":[{"name":"session id","in":"cookie","style":"form","explode":true,"schema":{"type":"array","items":{"type":"string"}}}],"responses":{}}}}}"#;
        let operation = parse_operations(
            specification,
            OpenApiFormat::Json,
            None,
            None,
            OpenApiLimits::default(),
        )
        .unwrap()
        .remove(0);
        let prepared = prepare_request(
            &operation,
            &serde_json::json!({"cookies":{"session id":["a b","x;y"]}}),
            &HeaderMap::with_capacity(0),
            HttpLimits::default(),
        )
        .unwrap();
        let origin = HttpOrigin::try_from_url(&operation.server).unwrap();
        let request = prepared
            .authorize(
                AuthorizedOrigin::new(origin, vec![IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))]).unwrap(),
            )
            .unwrap();
        assert_eq!(
            request.headers().get("cookie").unwrap().expose_secret(),
            "session%20id=a%20b&session%20id=x%3By"
        );
    }

    #[test]
    fn validates_required_properties_enums_and_basic_constraints() {
        let schema = serde_json::json!({
            "type":"object",
            "properties":{
                "mode":{"type":"string","enum":["safe"],"minLength":4,"maxLength":4},
                "count":{"type":"integer","minimum":1,"maximum":3}
            },
            "required":["mode","count"],
            "additionalProperties":false
        });
        assert!(validate_value(&serde_json::json!({"mode":"safe","count":2}), &schema).is_ok());
        for invalid in [
            serde_json::json!({"mode":"evil","count":2}),
            serde_json::json!({"mode":"safe"}),
            serde_json::json!({"mode":"safe","count":4}),
            serde_json::json!({"mode":"safe","count":2,"extra":true}),
        ] {
            assert!(validate_value(&invalid, &schema).is_err(), "{invalid}");
        }
    }

    #[test]
    fn rejects_dot_segments_in_path_parameters_and_arrays() {
        let scalar_operation = operation();
        for value in [serde_json::json!("."), serde_json::json!("..")] {
            let input = serde_json::json!({"path":{"id":value},"body":{}});
            assert!(
                prepare_request(
                    &scalar_operation,
                    &input,
                    &HeaderMap::with_capacity(0),
                    HttpLimits::default(),
                )
                .is_err()
            );
        }

        let mut operation = operation();
        operation.parameters[0].schema =
            serde_json::json!({"type":"array","items":{"type":"string"}});
        operation.parameters[0].explode = false;
        let input = serde_json::json!({"path":{"id":["safe",".."]},"body":{}});
        assert!(
            prepare_request(
                &operation,
                &input,
                &HeaderMap::with_capacity(0),
                HttpLimits::default(),
            )
            .is_err()
        );
    }
}
