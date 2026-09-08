//! JSON-RPC protocol decoding internals.

use serde::Serialize;
use serde_json::Value;

use super::{McpClientError, framing::serialize_bounded};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub(crate) enum JsonRpcId {
    Number(serde_json::Number),
    String(String),
}

#[derive(Debug, PartialEq)]
pub(crate) enum IncomingMessage {
    Response { id: u64, result: Value },
    Error { id: u64, code: i64 },
    Request { id: JsonRpcId, method: String },
    Notification,
}

pub(crate) fn encode_request(
    id: u64,
    method: &str,
    params: &Value,
    frame_limit: usize,
) -> Result<Vec<u8>, McpClientError> {
    serialize_outbound(
        &OutboundRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        },
        frame_limit,
    )
}

pub(crate) fn encode_notification(
    method: &str,
    params: &Value,
    frame_limit: usize,
) -> Result<Vec<u8>, McpClientError> {
    serialize_outbound(
        &OutboundNotification {
            jsonrpc: "2.0",
            method,
            params,
        },
        frame_limit,
    )
}

#[derive(Serialize)]
struct OutboundRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: &'a Value,
}

#[derive(Serialize)]
struct OutboundNotification<'a> {
    jsonrpc: &'static str,
    method: &'a str,
    params: &'a Value,
}

#[derive(Serialize)]
struct OutboundSuccess<'a> {
    jsonrpc: &'static str,
    id: &'a JsonRpcId,
    result: Value,
}

#[derive(Serialize)]
struct OutboundError<'a> {
    jsonrpc: &'static str,
    id: &'a JsonRpcId,
    error: OutboundErrorBody<'a>,
}

#[derive(Serialize)]
struct OutboundErrorBody<'a> {
    code: i64,
    message: &'a str,
}

pub(crate) fn encode_success_response(
    id: &JsonRpcId,
    frame_limit: usize,
) -> Result<Vec<u8>, McpClientError> {
    serialize_outbound(
        &OutboundSuccess {
            jsonrpc: "2.0",
            id,
            result: serde_json::Map::new().into(),
        },
        frame_limit,
    )
}

pub(crate) fn encode_error_response(
    id: &JsonRpcId,
    code: i64,
    message: &str,
    frame_limit: usize,
) -> Result<Vec<u8>, McpClientError> {
    serialize_outbound(
        &OutboundError {
            jsonrpc: "2.0",
            id,
            error: OutboundErrorBody { code, message },
        },
        frame_limit,
    )
}

fn serialize_outbound<T>(value: &T, frame_limit: usize) -> Result<Vec<u8>, McpClientError>
where
    T: Serialize + ?Sized,
{
    let payload_limit = frame_limit
        .checked_sub(1)
        .ok_or(McpClientError::MessageTooLarge { limit: frame_limit })?;
    serialize_bounded(value, payload_limit).map_err(|error| match error {
        McpClientError::MessageTooLarge { .. } => {
            McpClientError::MessageTooLarge { limit: frame_limit }
        }
        other => other,
    })
}

pub(crate) fn decode_incoming(frame: &[u8]) -> Result<IncomingMessage, McpClientError> {
    let value: Value = serde_json::from_slice(frame)
        .map_err(|_| McpClientError::Protocol("malformed_json".into()))?;
    let Value::Object(mut object) = value else {
        return Err(McpClientError::Protocol("message_not_object".into()));
    };
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(McpClientError::Protocol("invalid_jsonrpc_version".into()));
    }

    let id = object.remove("id");
    let result = object.remove("result");
    let error = object.remove("error");
    let method = object.remove("method");
    if let Some(method) = method {
        let method = method
            .as_str()
            .ok_or_else(|| McpClientError::Protocol("invalid_request_method".into()))?
            .to_owned();
        if result.is_some() || error.is_some() {
            return Err(McpClientError::Protocol("ambiguous_request_payload".into()));
        }
        return match id {
            Some(Value::Number(id)) => Ok(IncomingMessage::Request {
                id: JsonRpcId::Number(id),
                method,
            }),
            Some(Value::String(id)) => Ok(IncomingMessage::Request {
                id: JsonRpcId::String(id),
                method,
            }),
            Some(_) => Err(McpClientError::Protocol("invalid_request_id".into())),
            None => Ok(IncomingMessage::Notification),
        };
    }
    let id = id
        .as_ref()
        .and_then(Value::as_u64)
        .ok_or_else(|| McpClientError::Protocol("invalid_response_id".into()))?;
    match (result, error) {
        (Some(result), None) => Ok(IncomingMessage::Response { id, result }),
        (None, Some(error)) => {
            let code = error
                .get("code")
                .and_then(Value::as_i64)
                .ok_or_else(|| McpClientError::Protocol("invalid_error_response".into()))?;
            if !error.get("message").is_some_and(Value::is_string) {
                return Err(McpClientError::Protocol("invalid_error_response".into()));
            }
            Ok(IncomingMessage::Error { id, code })
        }
        _ => Err(McpClientError::Protocol(
            "ambiguous_response_payload".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use serde_json::json;

    use super::{
        IncomingMessage, JsonRpcId, decode_incoming, encode_error_response, encode_notification,
        encode_request, encode_success_response,
    };
    use crate::toolkits::mcp::McpClientError;

    #[test]
    fn encodes_requests_and_notifications_without_untrusted_text() {
        let params = json!({"cursor": "opaque"});
        let request = encode_request(42, "tools/list", &params, 1024).unwrap();
        let exact_frame_size = request.len() + 1;
        assert!(encode_request(42, "tools/list", &params, exact_frame_size).is_ok());
        assert!(matches!(
            encode_request(42, "tools/list", &params, exact_frame_size - 1),
            Err(McpClientError::MessageTooLarge { limit }) if limit == exact_frame_size - 1
        ));
        let request: serde_json::Value = serde_json::from_slice(&request).unwrap();
        assert_eq!(request["jsonrpc"], "2.0");
        assert_eq!(request["id"], 42);
        assert_eq!(request["method"], "tools/list");

        let notification =
            encode_notification("notifications/initialized", &json!({}), 1024).unwrap();
        let notification: serde_json::Value = serde_json::from_slice(&notification).unwrap();
        assert!(notification.get("id").is_none());
    }

    #[test]
    fn decodes_response_ids_notifications_and_safe_remote_errors() {
        assert_eq!(
            decode_incoming(br#"{"jsonrpc":"2.0","id":7,"result":{"ok":true}}"#).unwrap(),
            IncomingMessage::Response {
                id: 7,
                result: json!({"ok": true}),
            }
        );
        assert_eq!(
            decode_incoming(br#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#)
                .unwrap(),
            IncomingMessage::Notification
        );
        assert_eq!(
            decode_incoming(
                br#"{"jsonrpc":"2.0","id":9,"error":{"code":-32601,"message":"secret"}}"#
            )
            .unwrap(),
            IncomingMessage::Error {
                id: 9,
                code: -32601,
            }
        );
    }

    #[test]
    fn decodes_server_requests_and_encodes_bounded_replies() {
        assert_eq!(
            decode_incoming(br#"{"jsonrpc":"2.0","id":99,"method":"ping","params":{}}"#).unwrap(),
            IncomingMessage::Request {
                id: JsonRpcId::Number(99.into()),
                method: "ping".into(),
            }
        );
        let success = encode_success_response(&JsonRpcId::Number(99.into()), 128).unwrap();
        let success: serde_json::Value = serde_json::from_slice(&success).unwrap();
        assert_eq!(success["id"], 99);
        assert_eq!(success["result"], json!({}));
        let error = encode_error_response(
            &JsonRpcId::String("server-request".into()),
            -32601,
            "method not found",
            128,
        )
        .unwrap();
        let error: serde_json::Value = serde_json::from_slice(&error).unwrap();
        assert_eq!(error["id"], "server-request");
        assert_eq!(error["error"]["code"], -32601);
        assert_eq!(error["error"]["message"], "method not found");
    }

    #[test]
    fn accepts_string_server_request_ids_but_rejects_string_response_ids() {
        assert_eq!(
            decode_incoming(br#"{"jsonrpc":"2.0","id":"ping-1","method":"ping"}"#).unwrap(),
            IncomingMessage::Request {
                id: JsonRpcId::String("ping-1".into()),
                method: "ping".into(),
            }
        );
        assert_eq!(
            decode_incoming(br#"{"jsonrpc":"2.0","id":"","method":"ping"}"#).unwrap(),
            IncomingMessage::Request {
                id: JsonRpcId::String(String::new()),
                method: "ping".into(),
            }
        );
        assert!(
            decode_incoming(br#"{"jsonrpc":"2.0","id":"client-request","result":{}}"#).is_err()
        );
        let request = decode_incoming(br#"{"jsonrpc":"2.0","id":-7,"method":"ping"}"#).unwrap();
        assert_eq!(
            request,
            IncomingMessage::Request {
                id: JsonRpcId::Number((-7).into()),
                method: "ping".into(),
            }
        );
    }

    #[test]
    fn rejects_malformed_json_versions_and_ambiguous_responses() {
        for frame in [
            b"{".as_slice(),
            br#"{"jsonrpc":"1.0","id":1,"result":{}}"#,
            br#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1}}"#,
            br#"{"jsonrpc":"2.0","id":"one","result":{}}"#,
            br#"{"jsonrpc":"2.0","method":"notice","result":{}}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":-1}}"#,
            br#"{"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":7}}"#,
        ] {
            assert!(decode_incoming(frame).is_err());
        }
    }

    proptest! {
        #[test]
        fn response_id_is_correlated_exactly(id in any::<u64>()) {
            let frame = serde_json::to_vec(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {"ok": true}
            })).unwrap();
            prop_assert_eq!(
                decode_incoming(&frame).unwrap(),
                IncomingMessage::Response { id, result: json!({"ok": true}) }
            );
        }
    }
}
