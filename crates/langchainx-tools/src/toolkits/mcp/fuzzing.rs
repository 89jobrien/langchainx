//! Narrow entry points for fuzzing production MCP framing and protocol decoding.

use std::{
    cmp,
    pin::Pin,
    task::{Context, Poll},
};

use serde_json::Value;
use tokio::io::{AsyncRead, ReadBuf};

use super::{
    framing::{FrameBuffer, read_frame},
    protocol::{IncomingMessage, JsonRpcId, decode_incoming, encode_request},
};

/// Decoded MCP message details relevant to protocol invariants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum McpMessageSummary {
    /// A client response with its correlated request identifier.
    Response(u64),
    /// A client error response with its correlated request identifier and code.
    Error(u64, i64),
    /// A server request whose numeric identifier is preserved exactly.
    NumericRequest(String),
    /// A server request whose string identifier is preserved exactly.
    StringRequest(String),
    /// A notification without a request identifier.
    Notification,
}

/// Observable bounded effects of framing and decoding one fuzz input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpFuzzSummary {
    /// Decoded frame result, normalized for deterministic comparison.
    pub decoded: Result<McpMessageSummary, String>,
    /// Payload bytes returned by the frame reader, when a complete frame exists.
    pub frame_bytes: Option<usize>,
    /// Configured inbound frame bound, including the newline delimiter.
    pub frame_limit: usize,
    /// Encoded request bytes, when the outbound request fits its bound.
    pub outbound_bytes: Option<usize>,
    /// Whether an encoded request retained its exact numeric request identifier.
    pub request_id_preserved: bool,
}

/// Runs arbitrary bytes and read boundaries through production MCP framing and decoding.
///
/// The caller supplies a nonzero frame limit. The `chunks` bytes select short-read boundaries;
/// no protocol parsing is duplicated here. This entry point exists only behind `fuzzing`.
pub async fn exercise_mcp(input: &[u8], chunks: &[u8], frame_limit: usize) -> McpFuzzSummary {
    let frame_limit = frame_limit.max(1);
    let mut reader = ChunkedReader::new(input, chunks);
    let mut buffer = match FrameBuffer::new(frame_limit) {
        Ok(buffer) => buffer,
        Err(error) => {
            return McpFuzzSummary {
                decoded: Err(error.to_string()),
                frame_bytes: None,
                frame_limit,
                outbound_bytes: None,
                request_id_preserved: false,
            };
        }
    };
    let frame = read_frame(&mut reader, &mut buffer).await;
    let frame_bytes = frame.as_ref().ok().map(Vec::len);
    let decoded = frame
        .as_deref()
        .map_err(ToString::to_string)
        .and_then(|frame| decode_incoming(frame).map_err(|error| error.to_string()))
        .map(summarize);

    let request_id = input
        .get(..8)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u64::from_le_bytes)
        .unwrap_or(input.len() as u64);
    let params = frame
        .as_ref()
        .ok()
        .map(Vec::as_slice)
        .and_then(|frame| serde_json::from_slice::<Value>(frame).ok())
        .unwrap_or(Value::Null);
    let outbound_limit = frame_limit.min(4 * 1024);
    let outbound = encode_request(request_id, "fuzz", &params, outbound_limit).ok();
    let request_id_preserved = outbound.as_deref().is_some_and(|payload| {
        matches!(
            decode_incoming(payload),
            Ok(IncomingMessage::Request {
                id: JsonRpcId::Number(id),
                ..
            }) if id.as_u64() == Some(request_id)
        )
    });

    McpFuzzSummary {
        decoded,
        frame_bytes,
        frame_limit,
        outbound_bytes: outbound.as_ref().map(Vec::len),
        request_id_preserved,
    }
}

fn summarize(message: IncomingMessage) -> McpMessageSummary {
    match message {
        IncomingMessage::Response { id, .. } => McpMessageSummary::Response(id),
        IncomingMessage::Error { id, code } => McpMessageSummary::Error(id, code),
        IncomingMessage::Request {
            id: JsonRpcId::Number(id),
            ..
        } => McpMessageSummary::NumericRequest(id.to_string()),
        IncomingMessage::Request {
            id: JsonRpcId::String(id),
            ..
        } => McpMessageSummary::StringRequest(id),
        IncomingMessage::Notification => McpMessageSummary::Notification,
    }
}

struct ChunkedReader<'a> {
    input: &'a [u8],
    chunks: &'a [u8],
    offset: usize,
    chunk: usize,
}

impl<'a> ChunkedReader<'a> {
    fn new(input: &'a [u8], chunks: &'a [u8]) -> Self {
        Self {
            input,
            chunks,
            offset: 0,
            chunk: 0,
        }
    }
}

impl AsyncRead for ChunkedReader<'_> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.offset == self.input.len() {
            return Poll::Ready(Ok(()));
        }
        let requested = self
            .chunks
            .get(self.chunk % self.chunks.len().max(1))
            .copied()
            .map_or(1, |size| usize::from(size).max(1));
        let count = cmp::min(
            requested,
            cmp::min(buffer.remaining(), self.input.len() - self.offset),
        );
        buffer.put_slice(&self.input[self.offset..self.offset + count]);
        self.offset += count;
        self.chunk += 1;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::exercise_mcp;

    #[tokio::test]
    async fn exercise_is_deterministic_for_identical_input() {
        let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n";
        let first = exercise_mcp(input, &[1, 2, 3], 256).await;
        let second = exercise_mcp(input, &[1, 2, 3], 256).await;
        assert_eq!(first, second);
    }
}
