//! Bounded newline-delimited framing internals.

use std::io::{self, Write};

use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use super::McpClientError;

pub(crate) struct FrameBuffer {
    storage: Box<[u8]>,
    len: usize,
    limit: usize,
}

impl FrameBuffer {
    pub(crate) fn new(limit: usize) -> Result<Self, McpClientError> {
        let capacity = limit
            .checked_add(1)
            .ok_or(McpClientError::MessageTooLarge { limit })?;
        let mut storage = Vec::new();
        storage
            .try_reserve_exact(capacity)
            .map_err(|_| McpClientError::Protocol("frame_buffer_allocation_failed".into()))?;
        storage.resize(capacity, 0);
        Ok(Self {
            storage: storage.into_boxed_slice(),
            len: 0,
            limit,
        })
    }

    #[cfg(test)]
    fn capacity(&self) -> usize {
        self.storage.len()
    }

    #[cfg(test)]
    fn limit(&self) -> usize {
        self.limit
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.len
    }
}

pub(crate) async fn read_frame<R>(
    reader: &mut R,
    buffer: &mut FrameBuffer,
) -> Result<Vec<u8>, McpClientError>
where
    R: AsyncRead + Unpin,
{
    let limit = buffer.limit;
    loop {
        if let Some(newline) = buffer.storage[..buffer.len]
            .iter()
            .position(|byte| *byte == b'\n')
        {
            if newline.saturating_add(1) > limit {
                return Err(McpClientError::MessageTooLarge { limit });
            }
            let frame_end = if newline > 0 && buffer.storage[newline - 1] == b'\r' {
                newline - 1
            } else {
                newline
            };
            let frame = buffer.storage[..frame_end].to_vec();
            let remaining = buffer.len - newline - 1;
            buffer.storage.copy_within(newline + 1..buffer.len, 0);
            buffer.len = remaining;
            return Ok(frame);
        }
        if buffer.len > limit {
            return Err(McpClientError::MessageTooLarge { limit });
        }

        let remaining_with_sentinel = limit.saturating_add(1).saturating_sub(buffer.len);
        if remaining_with_sentinel == 0 {
            return Err(McpClientError::MessageTooLarge { limit });
        }
        let read = reader
            .read(&mut buffer.storage[buffer.len..buffer.len + remaining_with_sentinel])
            .await
            .map_err(|_| McpClientError::Disconnected)?;
        if read == 0 {
            return if buffer.len == 0 {
                Err(McpClientError::Disconnected)
            } else {
                Err(McpClientError::Protocol("truncated_frame".into()))
            };
        }
        buffer.len += read;
    }
}

pub(crate) fn serialized_size_bounded<T>(value: &T, limit: usize) -> Result<usize, McpClientError>
where
    T: Serialize + ?Sized,
{
    let mut writer = CountingWriter::new(limit);
    serde_json::to_writer(&mut writer, value).map_err(|_| writer.error())?;
    Ok(writer.written)
}

pub(crate) fn serialize_bounded<T>(value: &T, limit: usize) -> Result<Vec<u8>, McpClientError>
where
    T: Serialize + ?Sized,
{
    let size = serialized_size_bounded(value, limit)?;
    let mut writer = BoundedVecWriter {
        output: Vec::with_capacity(size),
        limit,
        overflowed: false,
    };
    if serde_json::to_writer(&mut writer, value).is_err() {
        return Err(if writer.overflowed {
            McpClientError::MessageTooLarge { limit }
        } else {
            McpClientError::Protocol("serialization_failed".into())
        });
    }
    Ok(writer.output)
}

struct CountingWriter {
    written: usize,
    limit: usize,
    overflowed: bool,
}

impl CountingWriter {
    fn new(limit: usize) -> Self {
        Self {
            written: 0,
            limit,
            overflowed: false,
        }
    }

    fn error(&self) -> McpClientError {
        if self.overflowed {
            McpClientError::MessageTooLarge { limit: self.limit }
        } else {
            McpClientError::Protocol("serialization_failed".into())
        }
    }
}

impl Write for CountingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let Some(total) = self.written.checked_add(bytes.len()) else {
            self.overflowed = true;
            return Err(io::Error::other("size_limit"));
        };
        if total > self.limit {
            self.overflowed = true;
            return Err(io::Error::other("size_limit"));
        }
        self.written = total;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct BoundedVecWriter {
    output: Vec<u8>,
    limit: usize,
    overflowed: bool,
}

impl Write for BoundedVecWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self
            .output
            .len()
            .checked_add(bytes.len())
            .is_none_or(|total| total > self.limit)
        {
            self.overflowed = true;
            return Err(io::Error::other("size_limit"));
        }
        self.output.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) async fn write_frame<W>(
    writer: &mut W,
    payload: &[u8],
    limit: usize,
) -> Result<(), McpClientError>
where
    W: AsyncWrite + Unpin,
{
    if payload.len().saturating_add(1) > limit {
        return Err(McpClientError::MessageTooLarge { limit });
    }
    writer
        .write_all(payload)
        .await
        .map_err(|_| McpClientError::Disconnected)?;
    writer
        .write_all(b"\n")
        .await
        .map_err(|_| McpClientError::Disconnected)?;
    writer
        .flush()
        .await
        .map_err(|_| McpClientError::Disconnected)
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncWriteExt, duplex};

    use serde_json::json;

    use super::{FrameBuffer, read_frame, serialize_bounded, write_frame};
    use crate::toolkits::mcp::McpClientError;

    #[tokio::test]
    async fn reads_partial_and_multiple_frames() {
        let (mut writer, mut reader) = duplex(128);
        writer.write_all(b"{\"first\":").await.unwrap();
        writer.write_all(b"1}\n{\"second\":2}\n").await.unwrap();
        let mut buffer = FrameBuffer::new(64).unwrap();
        assert_eq!(
            read_frame(&mut reader, &mut buffer).await.unwrap(),
            br#"{"first":1}"#
        );
        assert_eq!(
            read_frame(&mut reader, &mut buffer).await.unwrap(),
            br#"{"second":2}"#
        );
    }

    #[tokio::test]
    async fn rejects_oversized_frames_before_json_decoding() {
        let (mut writer, mut reader) = duplex(128);
        writer.write_all(b"123456789").await.unwrap();
        let mut buffer = FrameBuffer::new(8).unwrap();
        assert!(matches!(
            read_frame(&mut reader, &mut buffer).await,
            Err(McpClientError::MessageTooLarge { limit: 8 })
        ));
        assert_eq!(buffer.len(), 9, "reader may retain only one sentinel byte");
    }

    #[test]
    fn inbound_storage_has_one_fixed_sentinel_slot() {
        let buffer = FrameBuffer::new(8).unwrap();
        assert_eq!(buffer.capacity(), 9);
        assert_eq!(buffer.limit(), 8);
    }

    #[tokio::test]
    async fn accepts_exact_inbound_boundary_and_rejects_one_byte_over() {
        let (mut writer, mut reader) = duplex(128);
        writer.write_all(b"1234567\n12345678\n").await.unwrap();
        let mut buffer = FrameBuffer::new(8).unwrap();
        assert_eq!(
            read_frame(&mut reader, &mut buffer).await.unwrap(),
            b"1234567"
        );
        assert!(matches!(
            read_frame(&mut reader, &mut buffer).await,
            Err(McpClientError::MessageTooLarge { limit: 8 })
        ));
    }

    #[tokio::test]
    async fn distinguishes_clean_eof_from_a_truncated_frame() {
        let (writer, mut reader) = duplex(128);
        drop(writer);
        let mut buffer = FrameBuffer::new(64).unwrap();
        assert_eq!(
            read_frame(&mut reader, &mut buffer).await,
            Err(McpClientError::Disconnected)
        );

        let (mut writer, mut reader) = duplex(128);
        writer.write_all(b"partial").await.unwrap();
        drop(writer);
        let mut buffer = FrameBuffer::new(64).unwrap();
        assert!(matches!(
            read_frame(&mut reader, &mut buffer).await,
            Err(McpClientError::Protocol(message)) if message == "truncated_frame"
        ));
    }

    #[tokio::test]
    async fn writes_measured_complete_frames() {
        let (mut writer, mut reader) = duplex(128);
        assert!(matches!(
            write_frame(&mut writer, b"12345678", 8).await,
            Err(McpClientError::MessageTooLarge { limit: 8 })
        ));
        write_frame(&mut writer, b"{}", 8).await.unwrap();
        let mut buffer = FrameBuffer::new(8).unwrap();
        assert_eq!(read_frame(&mut reader, &mut buffer).await.unwrap(), b"{}");
    }

    #[test]
    fn counts_json_before_allocating_an_exact_bounded_buffer() {
        let value = json!({"value": "boundary"});
        let size = serde_json::to_vec(&value).unwrap().len();
        assert_eq!(serialize_bounded(&value, size).unwrap().len(), size);
        assert!(matches!(
            serialize_bounded(&value, size - 1),
            Err(McpClientError::MessageTooLarge { limit }) if limit == size - 1
        ));
    }

    #[tokio::test]
    async fn preserves_frames_at_every_chunk_boundary() {
        let frame = br#"{"jsonrpc":"2.0","id":7,"result":{}}"#;
        for split in 0..=frame.len() {
            let (mut writer, mut reader) = duplex(128);
            writer.write_all(&frame[..split]).await.unwrap();
            writer.write_all(&frame[split..]).await.unwrap();
            writer.write_all(b"\n").await.unwrap();
            let mut buffer = FrameBuffer::new(128).unwrap();
            assert_eq!(read_frame(&mut reader, &mut buffer).await.unwrap(), frame);
        }
    }
}
