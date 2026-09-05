#![cfg(all(feature = "mcp-toolkit", unix))]

use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::Duration,
};

use langchainx_tools::toolkits::mcp::{
    McpClient, McpClientError, McpStdioLimits, StdioMcpClient, StdioMcpConfig,
};
use secrecy::SecretString;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Barrier;

const FIXTURE: &str = r##"#!/bin/sh
mode=$1
pid_file=$2
descendant_file=$3
ready_fifo=$4
continue_fifo=$5
printf '%s' "$$" > "$pid_file"
IFS= read -r initialize || exit 1
case "$mode" in
  malformed)
    printf 'not-json\n'
    exit 0
    ;;
  timeout)
    /bin/sleep 2
    exit 0
    ;;
  overflow)
    i=0
    while [ "$i" -lt 200 ]; do
      printf x
      i=$((i + 1))
    done
    printf '\n'
    exit 0
    ;;
  unsupported_version)
    printf '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"unsupported","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}}\n'
    /bin/sleep 2
    exit 0
    ;;
  missing_tools_capability)
    printf '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{},"serverInfo":{"name":"fixture","version":"1"}}}\n'
    /bin/sleep 2
    exit 0
    ;;
esac
if [ "$mode" = ping ]; then
  printf '{"jsonrpc":"2.0","id":"ping-string","method":"ping","params":{}}\n'
  IFS= read -r ping_response || exit 1
  case "$ping_response" in
    *'"id":"ping-string"'*'"result":{}'*) ;;
    *) exit 19 ;;
  esac
  printf '{"jsonrpc":"2.0","id":98,"method":"unsupported/method","params":{}}\n'
  IFS= read -r method_response || exit 1
  case "$method_response" in
    *'"id":98'*'"code":-32601'*'"message":"method not found"'*) ;;
    *) exit 20 ;;
  esac
fi
printf '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fixture","version":"1"}}}\n'
IFS= read -r initialized || exit 1
if [ "$mode" = descendant ]; then
  /bin/sleep 30 &
  printf '%s' "$!" > "$descendant_file"
fi
if [ "$mode" = out_of_order ]; then
  IFS= read -r first || exit 1
  printf ready > "$ready_fifo"
  IFS= read -r second || exit 1
  printf '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"call"}],"isError":false}}\n'
  printf '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","inputSchema":{"type":"object","properties":{}}}]}}\n'
fi
if [ "$mode" = cancel_late ]; then
  IFS= read -r cancelled || exit 1
  printf ready > "$ready_fifo"
  IFS= read -r continue < "$continue_fifo" || exit 1
  IFS= read -r active || exit 1
  printf '{"jsonrpc":"2.0","id":2,"result":{"tools":[]}}\n'
  printf '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"active"}],"isError":false}}\n'
fi
if [ "$mode" = cancel_burst ]; then
  i=0
  while [ "$i" -lt 20 ]; do
    IFS= read -r cancelled || exit 1
    i=$((i + 1))
  done
  printf ready > "$ready_fifo"
  IFS= read -r continue < "$continue_fifo" || exit 1
  IFS= read -r active || exit 1
  i=2
  while [ "$i" -lt 22 ]; do
    printf '{"jsonrpc":"2.0","id":%s,"result":{"tools":[]}}\n' "$i"
    i=$((i + 1))
  done
  printf '{"jsonrpc":"2.0","id":22,"result":{"content":[{"type":"text","text":"active"}],"isError":false}}\n'
fi
if [ "$mode" = cancel_overflow ]; then
  IFS= read -r first || exit 1
  IFS= read -r second || exit 1
  IFS= read -r third || exit 1
  printf ready > "$ready_fifo"
  IFS= read -r continue < "$continue_fifo" || exit 1
  /bin/sleep 2
fi
while IFS= read -r line; do
  case "$line" in
    *'"method":"tools/list"'*)
      if [ "$mode" = invalid_list ]; then
        printf '{"jsonrpc":"2.0","id":2,"result":{"tools":"invalid"}}\n'
      else
        case "$line" in
          *'"cursor"'*)
            printf '{"jsonrpc":"2.0","id":2,"error":{"code":-32602,"message":"cursor must be absent"}}\n'
            ;;
          *)
            printf '{"jsonrpc":"2.0","id":2,"result":{"tools":[{"name":"echo","description":"untrusted","inputSchema":{"type":"object","properties":{"value":{"type":"string"}}}}]}}\n'
            ;;
        esac
      fi
      ;;
    *'"method":"tools/call"'*)
      if [ "$mode" = invalid_content ]; then
        printf '{"jsonrpc":"2.0","id":3,"result":{"content":{"type":"text","text":"invalid"},"isError":false}}\n'
      else
        printf '{"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"%s"},{"type":"text","text":"%s"}],"isError":false}}\n' "${CONFIGURED-unset}" "${HOME-unset}"
      fi
      ;;
    *'"method":"shutdown"'*|*'"method":"exit"'*)
      exit 17
      ;;
  esac
done
"##;

fn fixture(mode: &str) -> (TempDir, StdioMcpConfig, std::path::PathBuf) {
    let directory = TempDir::new().unwrap();
    let executable = directory.path().join("fixture.sh");
    let pid_file = directory.path().join("pid");
    let descendant_file = directory.path().join("descendant");
    let ready_fifo = directory.path().join("ready.fifo");
    let continue_fifo = directory.path().join("continue.fifo");
    nix::unistd::mkfifo(
        &ready_fifo,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .unwrap();
    nix::unistd::mkfifo(
        &continue_fifo,
        nix::sys::stat::Mode::S_IRUSR | nix::sys::stat::Mode::S_IWUSR,
    )
    .unwrap();
    fs::write(&executable, FIXTURE).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();
    let executable = fs::canonicalize(executable).unwrap();
    let working_directory = fs::canonicalize(directory.path()).unwrap();
    let config = StdioMcpConfig::new(executable)
        .with_args([
            mode,
            pid_file.to_str().unwrap(),
            descendant_file.to_str().unwrap(),
            ready_fifo.to_str().unwrap(),
            continue_fifo.to_str().unwrap(),
        ])
        .with_environment("CONFIGURED", SecretString::from("present"))
        .with_working_directory(working_directory);
    (directory, config, pid_file)
}

#[tokio::test]
async fn initializes_lists_calls_closes_stdio_and_isolates_environment() {
    let (_directory, config, pid_file) = fixture("success");
    let client = StdioMcpClient::connect(config).await.unwrap();
    let page = client.list_tools(None).await.unwrap();
    assert_eq!(page.tools.len(), 1);
    assert_eq!(page.tools[0].name, "echo");
    let result = client
        .call_tool("echo", json!({"value": "hello"}))
        .await
        .unwrap();
    assert_eq!(result.content[0]["text"], "present");
    assert_eq!(result.content[1]["text"], "unset");
    client.close().await.unwrap();
    client.close().await.unwrap();
    assert!(matches!(
        client.list_tools(None).await,
        Err(McpClientError::Disconnected)
    ));

    let pid = fs::read_to_string(pid_file).unwrap();
    assert!(
        !Command::new("/bin/kill")
            .args(["-0", pid.trim()])
            .status()
            .unwrap()
            .success()
    );
}

#[tokio::test]
async fn rejects_unsupported_negotiation_and_malformed_content() {
    for mode in ["unsupported_version", "missing_tools_capability"] {
        let (_directory, config, _) = fixture(mode);
        assert!(matches!(
            StdioMcpClient::connect(config).await,
            Err(McpClientError::Protocol(_))
        ));
    }

    let (_directory, config, _) = fixture("invalid_content");
    let client = StdioMcpClient::connect(config).await.unwrap();
    assert!(matches!(
        client.call_tool("echo", json!({})).await,
        Err(McpClientError::Protocol(_))
    ));
}

#[tokio::test]
async fn answers_server_ping_without_disconnecting() {
    let (_directory, config, _) = fixture("ping");
    let client = StdioMcpClient::connect(config).await.unwrap();
    assert_eq!(client.list_tools(None).await.unwrap().tools.len(), 1);
    client.close().await.unwrap();
}

#[tokio::test]
async fn correlates_out_of_order_replies_and_discards_cancelled_late_replies() {
    let (directory, config, _) = fixture("out_of_order");
    let client = StdioMcpClient::connect(config).await.unwrap();
    let list_client = client.clone();
    let listing = tokio::spawn(async move { list_client.list_tools(None).await });
    wait_for_fixture_ready(directory.path()).await;
    let call = client.call_tool("echo", json!({})).await.unwrap();
    assert_eq!(call.content[0]["text"], "call");
    assert_eq!(listing.await.unwrap().unwrap().tools[0].name, "echo");
    client.close().await.unwrap();

    let (directory, config, _) = fixture("cancel_late");
    let client = StdioMcpClient::connect(config).await.unwrap();
    let cancelled_client = client.clone();
    let cancelled = tokio::spawn(async move { cancelled_client.list_tools(None).await });
    wait_for_fixture_ready(directory.path()).await;
    cancelled.abort();
    let _ = cancelled.await;
    release_fixture(directory.path()).await;
    let call = client.call_tool("echo", json!({})).await.unwrap();
    assert_eq!(call.content[0]["text"], "active");
    client.close().await.unwrap();

    let (directory, config, _) = fixture("cancel_burst");
    let limits = McpStdioLimits {
        max_in_flight: 32,
        ..McpStdioLimits::default()
    };
    let client = StdioMcpClient::connect(config.with_limits(limits))
        .await
        .unwrap();
    let barrier = Arc::new(Barrier::new(21));
    let mut cancelled = Vec::new();
    for _ in 0..20 {
        let client = client.clone();
        let barrier = barrier.clone();
        cancelled.push(tokio::spawn(async move {
            barrier.wait().await;
            client.list_tools(None).await
        }));
    }
    barrier.wait().await;
    wait_for_fixture_ready(directory.path()).await;
    for request in &cancelled {
        request.abort();
    }
    for request in cancelled {
        let _ = request.await;
    }
    release_fixture(directory.path()).await;
    let call = client.call_tool("echo", json!({})).await.unwrap();
    assert_eq!(call.content[0]["text"], "active");
    client.close().await.unwrap();
}

#[tokio::test]
async fn cancellation_tombstone_overflow_fails_closed() {
    let (directory, config, _) = fixture("cancel_overflow");
    let limits = McpStdioLimits {
        max_in_flight: 4,
        cancelled_request_ids: 2,
        ..McpStdioLimits::default()
    };
    let client = StdioMcpClient::connect(config.with_limits(limits))
        .await
        .unwrap();
    let barrier = Arc::new(Barrier::new(4));
    let mut requests = Vec::new();
    for _ in 0..3 {
        let client = client.clone();
        let barrier = barrier.clone();
        requests.push(tokio::spawn(async move {
            barrier.wait().await;
            client.list_tools(None).await
        }));
    }
    barrier.wait().await;
    wait_for_fixture_ready(directory.path()).await;
    for request in &requests {
        request.abort();
    }
    for request in requests {
        let _ = request.await;
    }
    assert!(matches!(
        client.call_tool("echo", json!({})).await,
        Err(McpClientError::Disconnected)
    ));
    let _ = client.close().await;
}

#[tokio::test]
async fn close_is_concurrent_and_drop_reaps_processes_and_descendants() {
    let (_directory, config, pid_file) = fixture("success");
    let client = StdioMcpClient::connect(config).await.unwrap();
    let clone = client.clone();
    let (first, second) = tokio::join!(client.close(), clone.close());
    assert_eq!(first, second);
    assert_eq!(client.close().await, first);
    assert!(!process_is_alive(&fs::read_to_string(pid_file).unwrap()));

    let (directory, config, pid_file) = fixture("success");
    let client = StdioMcpClient::connect(config).await.unwrap();
    drop(client);
    wait_for_process_exit(&fs::read_to_string(pid_file).unwrap()).await;
    drop(directory);

    let (directory, config, _) = fixture("descendant");
    let client = StdioMcpClient::connect(config).await.unwrap();
    let descendant_file = directory.path().join("descendant");
    for _ in 0..50 {
        if descendant_file.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let descendant = fs::read_to_string(descendant_file).unwrap();
    client.close().await.unwrap();
    wait_for_process_exit(&descendant).await;
}

#[tokio::test]
async fn rejects_relative_noncanonical_executables_and_working_directories() {
    assert!(matches!(
        StdioMcpClient::connect(StdioMcpConfig::new("relative")).await,
        Err(McpClientError::Launch(_))
    ));
    let (directory, _, _) = fixture("success");
    let noncanonical = directory.path().join("missing/../fixture.sh");
    assert!(
        StdioMcpClient::connect(StdioMcpConfig::new(noncanonical))
            .await
            .is_err()
    );
    let executable = directory.path().join("fixture.sh");
    assert!(
        StdioMcpClient::connect(
            StdioMcpConfig::new(executable)
                .with_working_directory(directory.path().join("missing"))
        )
        .await
        .is_err()
    );
}

fn process_is_alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid.trim()])
        .status()
        .is_ok_and(|status| status.success())
}

async fn wait_for_fixture_ready(directory: &Path) {
    let path = directory.join("ready.fifo");
    tokio::task::spawn_blocking(move || {
        let mut signal = String::new();
        fs::File::open(path)
            .unwrap()
            .read_to_string(&mut signal)
            .unwrap();
        assert_eq!(signal, "ready");
    })
    .await
    .unwrap();
}

async fn release_fixture(directory: &Path) {
    let path: PathBuf = directory.join("continue.fifo");
    tokio::task::spawn_blocking(move || {
        fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .write_all(b"continue\n")
            .unwrap();
    })
    .await
    .unwrap();
}

async fn wait_for_process_exit(pid: &str) {
    for _ in 0..100 {
        if !process_is_alive(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!process_is_alive(pid), "process {pid} was not reaped");
}

#[tokio::test]
async fn terminates_on_malformed_timeout_and_oversized_frames() {
    let (_directory, config, _) = fixture("malformed");
    assert!(matches!(
        StdioMcpClient::connect(config).await,
        Err(McpClientError::Protocol(_))
    ));

    let (_directory, config, _) = fixture("timeout");
    let limits = McpStdioLimits {
        request_timeout: Duration::from_millis(25),
        close_timeout: Duration::from_millis(25),
        ..McpStdioLimits::default()
    };
    assert!(matches!(
        StdioMcpClient::connect(config.with_limits(limits)).await,
        Err(McpClientError::Timeout)
    ));

    let (_directory, config, _) = fixture("overflow");
    let limits = McpStdioLimits {
        inbound_frame_bytes: 64,
        ..McpStdioLimits::default()
    };
    assert!(matches!(
        StdioMcpClient::connect(config.with_limits(limits)).await,
        Err(McpClientError::MessageTooLarge { limit: 64 })
    ));

    let (_directory, config, _) = fixture("invalid_list");
    let client = StdioMcpClient::connect(config).await.unwrap();
    assert!(matches!(
        client.list_tools(None).await,
        Err(McpClientError::Protocol(_))
    ));
    assert!(matches!(
        client.call_tool("echo", json!({})).await,
        Err(McpClientError::Disconnected)
    ));
}
