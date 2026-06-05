//! Integration tests for [`MiniboxTool`] against a live `miniboxd`.
//!
//! These tests are gated behind the `MINIBOX_INTEGRATION_TESTS` environment variable.
//! Run them with:
//!
//! ```sh
//! MINIBOX_INTEGRATION_TESTS=1 cargo test -p langchainx-tools --test minibox_integration
//! ```
//!
//! Without the env var every test returns immediately (pass/skip).

use langchainx_tools::minibox::MiniboxTool;
use langchainx_tools::Tool;
use serde_json::json;

fn minibox_enabled() -> bool {
    std::env::var("MINIBOX_INTEGRATION_TESTS").is_ok()
}

/// Convenience: invoke the tool and unwrap, panicking with a clear message on failure.
async fn invoke(tool: &MiniboxTool, input: serde_json::Value) -> String {
    tool.run(input)
        .await
        .unwrap_or_else(|e| panic!("MiniboxTool::run failed: {e}"))
}

// ── lifecycle: run → ps → stop → rm ──────────────────────────────────────────

#[tokio::test]
async fn lifecycle_run_ps_stop_rm() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    // run a long-lived container in detached mode
    let run_out = invoke(
        &tool,
        json!({
            "action": "run",
            "image": "alpine",
            "tag": "latest",
            "name": "lcx-test-lifecycle",
            "command": ["sleep", "300"]
        }),
    )
    .await;

    // extract container id from run output (first token of the line)
    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(
        !container_id.is_empty(),
        "run should return a container id; got: {run_out}"
    );

    // ps — verify the container appears
    let ps_out = invoke(&tool, json!({ "action": "ps" })).await;
    assert!(
        ps_out.contains(&container_id) || ps_out.contains("lcx-test-lifecycle"),
        "ps output should contain the running container; got: {ps_out}"
    );

    // stop
    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;

    // rm
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// ── exec: run → exec → verify output → stop → rm ────────────────────────────

#[tokio::test]
async fn exec_returns_command_output() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    let run_out = invoke(
        &tool,
        json!({
            "action": "run",
            "image": "alpine",
            "tag": "latest",
            "name": "lcx-test-exec",
            "command": ["sleep", "300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    // exec a command and check output
    let exec_out = invoke(
        &tool,
        json!({
            "action": "exec",
            "container_id": container_id,
            "cmd": ["echo", "hello-langchainx"]
        }),
    )
    .await;

    assert!(
        exec_out.contains("hello-langchainx"),
        "exec output should contain 'hello-langchainx'; got: {exec_out}"
    );

    // cleanup
    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// ── logs: run → generate output → logs → verify content → stop → rm ─────────

#[tokio::test]
async fn logs_contains_container_output() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    // Run a container that prints a known string then stays alive.
    // The `sh -c` form lets us chain two commands.
    let run_out = invoke(
        &tool,
        json!({
            "action": "run",
            "image": "alpine",
            "tag": "latest",
            "name": "lcx-test-logs",
            "command": ["sh", "-c", "echo marker-langchainx-logs && sleep 300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    // Give the container a moment to produce output before fetching logs.
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let logs_out = invoke(
        &tool,
        json!({ "action": "logs", "id": container_id }),
    )
    .await;

    assert!(
        logs_out.contains("marker-langchainx-logs"),
        "logs should contain 'marker-langchainx-logs'; got: {logs_out}"
    );

    // cleanup
    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// ── pull: lightweight image ───────────────────────────────────────────────────

#[tokio::test]
async fn pull_alpine_image() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    // alpine:latest is a small (~3 MB) well-known image.
    let pull_out = invoke(
        &tool,
        json!({
            "action": "pull",
            "image": "alpine",
            "tag": "latest"
        }),
    )
    .await;

    // miniboxd should produce some acknowledgement on stdout/stderr.
    assert!(
        !pull_out.is_empty(),
        "pull should return non-empty output; got empty string"
    );
}
