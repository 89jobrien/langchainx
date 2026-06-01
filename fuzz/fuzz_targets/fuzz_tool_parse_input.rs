#![no_main]
use libfuzzer_sys::fuzz_target;

use langchainx::tools::Tool;
use serde_json::Value;

struct NoopTool;

#[async_trait::async_trait]
impl Tool for NoopTool {
    fn name(&self) -> String {
        "noop".into()
    }
    fn description(&self) -> String {
        "noop".into()
    }
    async fn run(&self, _input: Value) -> Result<String, langchainx::tools::ToolError> {
        Ok(String::new())
    }
}

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let tool = NoopTool;
            let _ = tool.parse_input(s).await;
            let _ = tool.call(s).await;
        });
    }
});
