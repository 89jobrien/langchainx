#![no_main]

use langchainx_tools::toolkits::mcp::fuzzing::exercise_mcp;
use libfuzzer_sys::fuzz_target;
use std::sync::OnceLock;

const MAX_FRAME_BYTES: usize = 4 * 1024;
const SEPARATOR: &[u8] = b"\n---CHUNKS---\n";

fuzz_target!(|data: &[u8]| {
    let (frame, chunks, limit) = split_input(data);
    let result = runtime().block_on(exercise_mcp(frame, chunks, limit));

    if let Some(frame_bytes) = result.frame_bytes {
        assert!(frame_bytes.saturating_add(1) <= result.frame_limit);
    }
    if let Some(outbound_bytes) = result.outbound_bytes {
        assert!(outbound_bytes.saturating_add(1) <= result.frame_limit.min(MAX_FRAME_BYTES));
        assert!(result.request_id_preserved);
    }
});

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime")
    })
}

fn split_input(data: &[u8]) -> (&[u8], &[u8], usize) {
    if let Some(separator) = data
        .windows(SEPARATOR.len())
        .position(|window| window == SEPARATOR)
    {
        return (
            &data[..separator],
            &data[separator + SEPARATOR.len()..],
            512,
        );
    }
    let limit = data
        .first()
        .map_or(1, |byte| usize::from(*byte) % MAX_FRAME_BYTES + 1);
    let payload = data.get(1..).unwrap_or_default();
    let split = payload
        .first()
        .map_or(0, |selector| usize::from(*selector) % (payload.len() + 1));
    (&payload[..split], &payload[split..], limit)
}
