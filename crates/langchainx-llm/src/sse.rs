use crate::language_models::LLMError;

#[derive(Debug, Default)]
pub(crate) struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    pub(crate) fn push(&mut self, bytes: &[u8]) -> Result<Vec<String>, LLMError> {
        self.buffer.extend_from_slice(bytes);
        let mut events = Vec::new();

        while let Some((end, separator_len)) = event_boundary(&self.buffer) {
            let frame = self.buffer.drain(..end + separator_len).collect::<Vec<_>>();
            let frame = std::str::from_utf8(&frame[..end])
                .map_err(|error| LLMError::ParsingError(error.to_string()))?;
            let data = frame
                .lines()
                .filter_map(|line| line.strip_prefix("data:").map(str::trim_start))
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() && data != "[DONE]" {
                events.push(data);
            }
        }

        Ok(events)
    }

    pub(crate) fn finish(self) -> Result<(), LLMError> {
        if self.buffer.iter().all(u8::is_ascii_whitespace) {
            Ok(())
        } else {
            Err(LLMError::ParsingError(
                "incomplete server-sent event at end of stream".to_string(),
            ))
        }
    }
}

fn event_boundary(buffer: &[u8]) -> Option<(usize, usize)> {
    let lf = buffer.windows(2).position(|window| window == b"\n\n");
    let crlf = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    match (lf, crlf) {
        (Some(left), Some(right)) if left <= right => Some((left, 2)),
        (Some(_), Some(right)) => Some((right, 4)),
        (Some(left), None) => Some((left, 2)),
        (None, Some(right)) => Some((right, 4)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffers_fragmented_json_and_utf8() {
        let mut decoder = SseDecoder::default();
        let payload = b"data: {\"content\":\"caf\xc3\xa9\"}\n\n";
        let split = payload.iter().position(|byte| *byte == 0xc3).unwrap() + 1;

        assert!(decoder.push(&payload[..split]).unwrap().is_empty());
        assert_eq!(
            decoder.push(&payload[split..]).unwrap(),
            vec!["{\"content\":\"caf\u{e9}\"}".to_string()]
        );
        decoder.finish().unwrap();
    }

    #[test]
    fn emits_multiple_events_and_ignores_done() {
        let mut decoder = SseDecoder::default();
        let events = decoder
            .push(b"data: {\"one\":1}\n\ndata: {\"two\":2}\n\ndata: [DONE]\n\n")
            .unwrap();

        assert_eq!(events, vec!["{\"one\":1}", "{\"two\":2}"]);
    }
}
