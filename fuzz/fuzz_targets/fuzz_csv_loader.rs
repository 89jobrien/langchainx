#![no_main]
use libfuzzer_sys::fuzz_target;

use futures_util::StreamExt;
use langchainx_loaders::{CsvLoader, Loader};

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let loader = CsvLoader::from_string(s, vec![]);
            if let Ok(stream) = loader.load().await {
                tokio::pin!(stream);
                while let Some(_item) = stream.next().await {}
            }
        });
    }
});
