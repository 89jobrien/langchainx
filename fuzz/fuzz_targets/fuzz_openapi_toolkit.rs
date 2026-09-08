#![no_main]

use langchainx_tools::toolkits::openapi::{
    fuzzing::{exercise_openapi, FUZZ_LIMITS},
    OpenApiFormat,
};
use libfuzzer_sys::fuzz_target;

const SEPARATOR: &[u8] = b"\n---ARGS---\n";

fuzz_target!(|data: &[u8]| {
    let (format, specification, arguments) = split_input(data);
    let first = exercise_openapi(specification, arguments, format);
    let second = exercise_openapi(specification, arguments, format);
    assert_eq!(first, second, "rejection and mapping must be deterministic");

    if let Ok(summary) = first {
        assert!(summary.operations <= FUZZ_LIMITS.operations);
        assert!(summary.prepared_requests <= summary.operations);
        assert!(summary.maximum_url_bytes <= FUZZ_LIMITS.http.url_bytes);
        assert!(summary.maximum_header_bytes <= FUZZ_LIMITS.http.header_bytes);
        assert!(summary.maximum_body_bytes <= FUZZ_LIMITS.http.request_body_bytes);
        assert!(summary.origins_preserved);
    }
});

fn split_input(data: &[u8]) -> (OpenApiFormat, &[u8], &[u8]) {
    let (format, payload) = match data.split_first() {
        Some((b'Y', payload)) => (OpenApiFormat::Yaml, payload),
        Some((_, payload)) => (OpenApiFormat::Json, payload),
        None => (OpenApiFormat::Json, data),
    };
    if let Some(separator) = payload
        .windows(SEPARATOR.len())
        .position(|window| window == SEPARATOR)
    {
        return (
            format,
            &payload[..separator],
            &payload[separator + SEPARATOR.len()..],
        );
    }
    let split = payload
        .first()
        .map_or(0, |selector| usize::from(*selector) % (payload.len() + 1));
    (format, &payload[..split], &payload[split..])
}
