#[allow(unused_imports)]
pub use langchainx_testsuite::fakes::{EchoTool, FakeEmbedder, FakeLLM, ScriptedAgent};

// ---------------------------------------------------------------------------
// Ollama availability check — skip tests when Ollama is not running
// ---------------------------------------------------------------------------

#[allow(dead_code)]
/// Returns `true` if Ollama is reachable and `model` is available locally.
/// Call at the top of any Ollama-tier test: `if !ollama_available("qwen2.5:0.5b").await { return; }`.
pub async fn ollama_available(model: &str) -> bool {
    let Ok(output) = tokio::process::Command::new("ollama")
        .args(["list"])
        .output()
        .await
    else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    // Match on word boundary: each line starts with "name:tag" followed by whitespace.
    // Use line-prefix matching to avoid "nomic-embed-text" matching "nomic-embed-text-v2-moe".
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().any(|line| {
        let name = line.split_whitespace().next().unwrap_or("");
        // Strip optional ":tag" suffix for bare-name matches (e.g. "qwen2.5:0.5b" exact)
        name == model || name.starts_with(&format!("{model}:"))
    })
}
