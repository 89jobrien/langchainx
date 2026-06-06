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

/// How long to wait for container output before fetching logs.
const LOG_SETTLE_MS: u64 = 500;

fn minibox_enabled() -> bool {
    std::env::var("MINIBOX_INTEGRATION_TESTS").is_ok()
}

// qual:allow(complexity) reason: "integration test helper -- panic is intentional"
async fn invoke(tool: &MiniboxTool, input: serde_json::Value) -> String {
    tool.run(input)
        .await
        .unwrap_or_else(|e| panic!("MiniboxTool::run failed: {e}"))
}

// -- lifecycle: run -> ps -> stop -> rm --------------------------------------

#[tokio::test]
async fn lifecycle_run_ps_stop_rm() {
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
            "name": "lcx-test-lifecycle",
            "command": ["sleep", "300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(
        !container_id.is_empty(),
        "run should return a container id; got: {run_out}"
    );

    let ps_out = invoke(&tool, json!({ "action": "ps" })).await;
    assert!(
        ps_out.contains(&container_id) || ps_out.contains("lcx-test-lifecycle"),
        "ps output should contain the running container; got: {ps_out}"
    );

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// -- exec: run -> exec -> verify output -> stop -> rm ------------------------

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

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// -- logs: run -> generate output -> logs -> verify content -> stop -> rm -----

#[tokio::test]
async fn logs_contains_container_output() {
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
            "name": "lcx-test-logs",
            "command": ["sh", "-c", "echo marker-langchainx-logs && sleep 300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    tokio::time::sleep(std::time::Duration::from_millis(LOG_SETTLE_MS)).await;

    let logs_out = invoke(
        &tool,
        json!({ "action": "logs", "id": container_id }),
    )
    .await;

    assert!(
        logs_out.contains("marker-langchainx-logs"),
        "logs should contain 'marker-langchainx-logs'; got: {logs_out}"
    );

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// -- minibox_enabled ---------------------------------------------------------

#[test]
fn minibox_enabled_returns_true_when_set() {
    unsafe { std::env::set_var("MINIBOX_INTEGRATION_TESTS", "1") };
    assert!(minibox_enabled());
    unsafe { std::env::remove_var("MINIBOX_INTEGRATION_TESTS") };
}

// -- pull: lightweight image -------------------------------------------------

#[tokio::test]
async fn pull_alpine_image() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    let pull_out = invoke(
        &tool,
        json!({
            "action": "pull",
            "image": "alpine",
            "tag": "latest"
        }),
    )
    .await;

    assert!(
        !pull_out.is_empty(),
        "pull should return non-empty output; got empty string"
    );
}

// -- pause and resume --------------------------------------------------------

#[tokio::test]
async fn pause_and_resume() {
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
            "name": "lcx-test-pause",
            "command": ["sleep", "300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    invoke(&tool, json!({ "action": "pause", "id": container_id })).await;

    invoke(&tool, json!({ "action": "unpause", "id": container_id })).await;

    let exec_out = invoke(
        &tool,
        json!({
            "action": "exec",
            "container_id": container_id,
            "cmd": ["echo", "resumed"]
        }),
    )
    .await;

    assert!(
        exec_out.contains("resumed"),
        "exec after resume should return 'resumed'; got: {exec_out}"
    );

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// -- rmi removes pulled image ------------------------------------------------

#[tokio::test]
async fn rmi_removes_pulled_image() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    invoke(
        &tool,
        json!({
            "action": "pull",
            "image": "alpine",
            "tag": "latest"
        }),
    )
    .await;

    let rmi_out = invoke(
        &tool,
        json!({
            "action": "rmi",
            "image_ref": "alpine:latest"
        }),
    )
    .await;

    assert!(
        !rmi_out.is_empty(),
        "rmi should return non-empty output; got empty string"
    );
}

// -- prune dry run -----------------------------------------------------------

#[tokio::test]
async fn prune_dry_run() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    let prune_out = invoke(
        &tool,
        json!({
            "action": "prune",
            "dry_run": true
        }),
    )
    .await;

    assert!(
        !prune_out.is_empty(),
        "prune dry_run should return non-empty output; got empty string"
    );
}

// -- stop nonexistent container errors ---------------------------------------

#[tokio::test]
async fn stop_nonexistent_container_errors() {
    if !minibox_enabled() {
        return;
    }

    let tool = MiniboxTool::new();

    let result = tool
        .run(json!({ "action": "stop", "id": "lcx-nonexistent-bogus-container-id" }))
        .await;

    assert!(
        result.is_err(),
        "stopping a nonexistent container should return Err"
    );
}

// -- exec on stopped container errors ----------------------------------------

#[tokio::test]
async fn exec_on_stopped_container_errors() {
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
            "name": "lcx-test-exec-stopped",
            "command": ["sleep", "300"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;

    let exec_result = tool
        .run(json!({
            "action": "exec",
            "container_id": container_id,
            "cmd": ["echo", "should-fail"]
        }))
        .await;

    assert!(
        exec_result.is_err(),
        "exec on a stopped container should return Err"
    );

    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}

// -- run with env vars -------------------------------------------------------

#[tokio::test]
async fn run_with_env_vars() {
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
            "name": "lcx-test-envvars",
            "command": ["sleep", "300"],
            "env": ["FOO=bar"]
        }),
    )
    .await;

    let container_id = run_out.split_whitespace().next().unwrap_or("").to_string();
    assert!(!container_id.is_empty(), "run should return a container id");

    let exec_out = invoke(
        &tool,
        json!({
            "action": "exec",
            "container_id": container_id,
            "cmd": ["env"]
        }),
    )
    .await;

    assert!(
        exec_out.contains("FOO=bar"),
        "exec output should contain 'FOO=bar'; got: {exec_out}"
    );

    invoke(&tool, json!({ "action": "stop", "id": container_id })).await;
    invoke(&tool, json!({ "action": "rm", "id": container_id })).await;
}
