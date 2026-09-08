use langchainx_core::tools::{Tool, ToolError};
use serde_json::Value;

/// A side-effect-free tool that includes its parsed input in the result.
#[derive(Clone, Copy, Debug, Default)]
pub struct EchoTool;

impl Tool for EchoTool {
    fn name(&self) -> String {
        "echo".into()
    }

    fn description(&self) -> String {
        "echoes input".into()
    }

    async fn run(&self, input: Value) -> Result<String, ToolError> {
        Ok(format!("echoed: {input}"))
    }
}
