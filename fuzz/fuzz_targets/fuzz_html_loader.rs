#![no_main]
use libfuzzer_sys::fuzz_target;

use futures_util::StreamExt;
use langchainx_loaders::{HtmlLoader, Loader};
use std::io::Cursor;
use url::Url;

fuzz_target!(|data: &[u8]| {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let url = Url::parse("https://example.com/").unwrap();
        let loader = HtmlLoader::new(Cursor::new(data.to_vec()), url);
        if let Ok(stream) = loader.load().await {
            tokio::pin!(stream);
            while let Some(_item) = stream.next().await {}
        }
    });
});
