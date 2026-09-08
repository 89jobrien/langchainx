#![no_main]
use libfuzzer_sys::fuzz_target;

use langchainx::output_parsers::{MarkdownParser, OutputParser};

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let parser = MarkdownParser::new();
            let _ = parser.parse(input).await;
        });
    }
});
