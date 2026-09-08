use std::{
    collections::{BTreeMap, HashMap, HashSet, hash_map::Entry},
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        Arc, Mutex as StdMutex, Weak,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use tokio::{
    process::{Child, ChildStdin, Command},
    sync::{Mutex, Notify, Semaphore, oneshot},
    task::JoinHandle,
};

use super::{
    McpClient, McpClientError, McpStdioLimits, McpToolPage, McpToolResult,
    client::validate_content_blocks,
    framing::{FrameBuffer, read_frame, write_frame},
    protocol::{IncomingMessage, JsonRpcId, decode_incoming, encode_notification, encode_request},
};

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";
type PendingSender = oneshot::Sender<Result<Value, McpClientError>>;
type CloseResult = Result<(), McpClientError>;

/// Configuration for launching a local MCP server over stdio.
#[derive(Clone)]
pub struct StdioMcpConfig {
    executable: PathBuf,
    args: Vec<OsString>,
    environment: BTreeMap<OsString, SecretString>,
    working_directory: Option<PathBuf>,
    limits: McpStdioLimits,
}

impl StdioMcpConfig {
    /// Creates a configuration for `executable` with secure default limits.
    pub fn new(executable: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            environment: BTreeMap::new(),
            working_directory: None,
            limits: McpStdioLimits::default(),
        }
    }

    /// Appends one argument without shell interpretation.
    pub fn with_arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Appends arguments without shell interpretation.
    pub fn with_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Adds one environment entry to the otherwise empty child environment.
    pub fn with_environment(mut self, name: impl Into<OsString>, value: SecretString) -> Self {
        self.environment.insert(name.into(), value);
        self
    }

    /// Sets the canonical controlled working directory used by the child.
    pub fn with_working_directory(mut self, path: impl Into<PathBuf>) -> Self {
        self.working_directory = Some(path.into());
        self
    }

    /// Replaces stdio transport limits.
    pub fn with_limits(mut self, limits: McpStdioLimits) -> Self {
        self.limits = limits;
        self
    }
}

struct ProcessGroup {
    id: StdMutex<Option<u32>>,
}

impl ProcessGroup {
    fn new(id: Option<u32>) -> Self {
        Self {
            id: StdMutex::new(id),
        }
    }

    fn take(&self) -> Option<u32> {
        self.id.lock().ok().and_then(|mut id| id.take())
    }
}

struct CanceledIds {
    ids: HashSet<u64>,
    limit: usize,
}

impl CanceledIds {
    fn new(limit: usize) -> Self {
        Self {
            ids: HashSet::new(),
            limit,
        }
    }

    fn insert(&mut self, id: u64) -> Result<(), McpClientError> {
        if self.ids.contains(&id) {
            return Ok(());
        }
        if self.ids.len() >= self.limit {
            return Err(McpClientError::CancellationLimitExceeded { limit: self.limit });
        }
        self.ids.insert(id);
        Ok(())
    }

    fn remove(&mut self, id: u64) -> bool {
        self.ids.remove(&id)
    }
}

struct RequestRegistry {
    pending: HashMap<u64, PendingSender>,
    canceled: CanceledIds,
}

impl RequestRegistry {
    fn new(cancelled_request_ids: usize) -> Self {
        Self {
            pending: HashMap::new(),
            canceled: CanceledIds::new(cancelled_request_ids),
        }
    }
}

async fn lock_before<'a, T>(
    lock: &'a Mutex<T>,
    deadline: tokio::time::Instant,
) -> Result<tokio::sync::MutexGuard<'a, T>, McpClientError> {
    tokio::time::timeout_at(deadline, lock.lock())
        .await
        .map_err(|_| McpClientError::Timeout)
}

struct StdioState {
    writer: Mutex<Option<ChildStdin>>,
    child: Mutex<Option<Child>>,
    process_group: ProcessGroup,
    requests: StdMutex<RequestRegistry>,
    semaphore: Arc<Semaphore>,
    next_id: AtomicU64,
    terminal: AtomicBool,
    limits: McpStdioLimits,
    reader_task: StdMutex<Option<JoinHandle<()>>>,
    close_started: AtomicBool,
    close_result: StdMutex<Option<CloseResult>>,
    close_notify: Notify,
}

impl StdioState {
    fn fail_pending(&self, error: McpClientError) {
        if let Ok(mut requests) = self.requests.lock() {
            for (_, sender) in requests.pending.drain() {
                let _ = sender.send(Err(error.clone()));
            }
        }
    }

    async fn terminate(self: &Arc<Self>, error: McpClientError, abort_reader: bool) {
        self.terminal.store(true, Ordering::Release);
        self.fail_pending(error);
        let deadline = tokio::time::Instant::now() + self.limits.close_timeout;
        if let Ok(mut writer) = lock_before(&self.writer, deadline).await {
            writer.take();
        }
        let (mut child, deferred_reap) = match lock_before(&self.child, deadline).await {
            Ok(mut child) => (child.take(), false),
            Err(_) => (None, true),
        };
        let process_group_id = self.process_group.take();
        kill_process_group(process_group_id);
        let reap_later = if let Some(process) = child.as_mut() {
            let _ = process.start_kill();
            tokio::time::timeout_at(deadline, process.wait())
                .await
                .is_err()
        } else {
            false
        };
        if reap_later && let Some(mut process) = child.take() {
            tokio::spawn(async move {
                let _ = process.wait().await;
            });
        }
        if deferred_reap {
            self.spawn_deferred_reaper();
        }
        kill_process_group(process_group_id);
        if abort_reader
            && let Ok(mut task) = self.reader_task.lock()
            && let Some(task) = task.take()
        {
            task.abort();
        }
    }

    async fn close_process(self: &Arc<Self>) -> CloseResult {
        self.terminal.store(true, Ordering::Release);
        let deadline = tokio::time::Instant::now() + self.limits.close_timeout;
        let mut writer = match lock_before(&self.writer, deadline).await {
            Ok(writer) => writer,
            Err(error) => return self.close_lock_failure(error),
        };
        writer.take();
        drop(writer);
        let mut child = match lock_before(&self.child, deadline).await {
            Ok(mut child) => child.take(),
            Err(error) => return self.close_lock_failure(error),
        };
        let process_group_id = self.process_group.take();
        let result = if let Some(child) = child.as_mut() {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let graceful_deadline = tokio::time::Instant::now() + remaining / 2;
            match tokio::time::timeout_at(graceful_deadline, child.wait()).await {
                Ok(Ok(_)) => Ok(()),
                Ok(Err(_)) => Err(McpClientError::Disconnected),
                Err(_) => {
                    kill_process_group(process_group_id);
                    let _ = child.start_kill();
                    match tokio::time::timeout_at(deadline, child.wait()).await {
                        Ok(_) => Ok(()),
                        Err(_) => Err(McpClientError::Timeout),
                    }
                }
            }
        } else {
            Ok(())
        };
        if matches!(result, Err(McpClientError::Timeout))
            && let Some(mut child) = child.take()
        {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
        }
        kill_process_group(process_group_id);
        self.fail_pending(McpClientError::Disconnected);
        if let Ok(mut task) = self.reader_task.lock()
            && let Some(task) = task.take()
        {
            task.abort();
        }
        result
    }

    fn close_lock_failure(self: &Arc<Self>, error: McpClientError) -> CloseResult {
        let process_group_id = self.process_group.take();
        kill_process_group(process_group_id);
        self.fail_pending(McpClientError::Disconnected);
        self.spawn_deferred_reaper();
        Err(error)
    }

    fn spawn_deferred_reaper(self: &Arc<Self>) {
        let state = self.clone();
        tokio::spawn(async move {
            state.writer.lock().await.take();
            let mut child = state.child.lock().await.take();
            if let Some(child) = child.as_mut() {
                let _ = child.start_kill();
                let _ = child.wait().await;
            }
        });
    }

    fn publish_close(&self, result: CloseResult) {
        if let Ok(mut stored) = self.close_result.lock() {
            *stored = Some(result);
        }
        self.close_notify.notify_waiters();
    }

    async fn wait_for_close(&self) -> CloseResult {
        loop {
            let notified = self.close_notify.notified();
            if let Ok(result) = self.close_result.lock()
                && let Some(result) = result.clone()
            {
                return result;
            }
            notified.await;
        }
    }
}

impl Drop for StdioState {
    fn drop(&mut self) {
        kill_process_group(self.process_group.take());
    }
}

/// MCP client backed by one contained local subprocess.
#[derive(Clone)]
pub struct StdioMcpClient {
    state: Arc<StdioState>,
}

impl StdioMcpClient {
    /// Launches and initializes the configured MCP server.
    ///
    /// # Errors
    ///
    /// Returns [`McpClientError`] for invalid limits or paths, launch failures, bounded framing
    /// failures, initialization timeouts, or unsupported protocol capabilities.
    pub async fn connect(config: StdioMcpConfig) -> Result<Self, McpClientError> {
        config.limits.validate()?;
        let executable = require_canonical_file(&config.executable, "invalid_executable")?;
        let working_directory = match config.working_directory.as_deref() {
            Some(path) => require_canonical_directory(path)?,
            None => executable
                .parent()
                .ok_or_else(|| McpClientError::Launch("invalid_working_directory".into()))?
                .to_path_buf(),
        };

        let mut command = Command::new(&executable);
        command
            .args(&config.args)
            .current_dir(working_directory)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        for (name, value) in &config.environment {
            command.env(name, value.expose_secret());
        }
        contain_process(&mut command);

        let mut child = command
            .spawn()
            .map_err(|_| McpClientError::Launch("spawn_failed".into()))?;
        let process_group_id = child.id();
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpClientError::Launch("stdin_unavailable".into()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpClientError::Launch("stdout_unavailable".into()))?;
        let state = Arc::new(StdioState {
            writer: Mutex::new(Some(stdin)),
            child: Mutex::new(Some(child)),
            process_group: ProcessGroup::new(process_group_id),
            requests: StdMutex::new(RequestRegistry::new(config.limits.cancelled_request_ids)),
            semaphore: Arc::new(Semaphore::new(config.limits.max_in_flight)),
            next_id: AtomicU64::new(1),
            terminal: AtomicBool::new(false),
            limits: config.limits,
            reader_task: StdMutex::new(None),
            close_started: AtomicBool::new(false),
            close_result: StdMutex::new(None),
            close_notify: Notify::new(),
        });
        let task = tokio::spawn(reader_loop(Arc::downgrade(&state), stdout));
        state
            .reader_task
            .lock()
            .map_err(|_| McpClientError::Launch("state_unavailable".into()))?
            .replace(task);
        let client = Self { state };

        let initialized = client
            .request(
                "initialize",
                json!({
                    "protocolVersion": SUPPORTED_PROTOCOL_VERSION,
                    "capabilities": {},
                    "clientInfo": {"name": "langchainx", "version": env!("CARGO_PKG_VERSION")}
                }),
                config.limits.request_timeout,
            )
            .await;
        let initialized = match initialized {
            Ok(value) => value,
            Err(error) => {
                client.state.terminate(error.clone(), true).await;
                return Err(error);
            }
        };
        if !valid_initialize_response(&initialized) {
            let error = McpClientError::Protocol("unsupported_initialize_response".into());
            client.state.terminate(error.clone(), true).await;
            return Err(error);
        }
        if let Err(error) = client
            .notify(
                "notifications/initialized",
                json!({}),
                config.limits.request_timeout,
            )
            .await
        {
            client.state.terminate(error.clone(), true).await;
            return Err(error);
        }
        Ok(client)
    }

    async fn request(
        &self,
        method: &str,
        params: Value,
        timeout: std::time::Duration,
    ) -> Result<Value, McpClientError> {
        match tokio::time::timeout(timeout, self.request_exchange(method, params)).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => {
                if matches!(
                    error,
                    McpClientError::Disconnected
                        | McpClientError::MessageTooLarge { .. }
                        | McpClientError::RequestIdCollision
                        | McpClientError::RequestIdExhausted
                ) {
                    self.state.terminate(error.clone(), true).await;
                }
                Err(error)
            }
            Err(_) => {
                self.state.terminate(McpClientError::Timeout, true).await;
                Err(McpClientError::Timeout)
            }
        }
    }

    async fn request_exchange(&self, method: &str, params: Value) -> Result<Value, McpClientError> {
        if self.state.terminal.load(Ordering::Acquire) {
            return Err(McpClientError::Disconnected);
        }
        let permit = self
            .state
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| McpClientError::Disconnected)?;
        if self.state.terminal.load(Ordering::Acquire) {
            return Err(McpClientError::Disconnected);
        }

        let id = next_request_id(&self.state.next_id)?;
        let payload = encode_request(id, method, &params, self.state.limits.outbound_frame_bytes)?;
        let (sender, receiver) = oneshot::channel();
        insert_pending(&self.state.requests, id, sender)?;
        let mut pending_guard = PendingGuard::new(self.state.clone(), id);
        {
            let mut writer = self.state.writer.lock().await;
            let writer = writer.as_mut().ok_or(McpClientError::Disconnected)?;
            write_frame(writer, &payload, self.state.limits.outbound_frame_bytes).await
        }?;

        let response = receiver.await;
        drop(permit);
        match response {
            Ok(result) => {
                pending_guard.disarm();
                result
            }
            Err(_) => Err(McpClientError::Disconnected),
        }
    }

    async fn notify(
        &self,
        method: &str,
        params: Value,
        timeout: std::time::Duration,
    ) -> Result<(), McpClientError> {
        if self.state.terminal.load(Ordering::Acquire) {
            return Err(McpClientError::Disconnected);
        }
        let payload = encode_notification(method, &params, self.state.limits.outbound_frame_bytes)?;
        let write_result = {
            let mut writer = self.state.writer.lock().await;
            let writer = writer.as_mut().ok_or(McpClientError::Disconnected)?;
            tokio::time::timeout(
                timeout,
                write_frame(writer, &payload, self.state.limits.outbound_frame_bytes),
            )
            .await
        };
        match write_result {
            Ok(Ok(())) => Ok(()),
            Ok(Err(error)) => {
                self.state.terminate(error.clone(), true).await;
                Err(error)
            }
            Err(_) => {
                self.state.terminate(McpClientError::Timeout, true).await;
                Err(McpClientError::Timeout)
            }
        }
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn list_tools(&self, cursor: Option<String>) -> Result<McpToolPage, McpClientError> {
        let params = cursor.map_or_else(|| json!({}), |cursor| json!({"cursor": cursor}));
        let result = self
            .request("tools/list", params, self.state.limits.request_timeout)
            .await?;
        match serde_json::from_value(result) {
            Ok(page) => Ok(page),
            Err(_) => {
                let error = McpClientError::Protocol("invalid_tools_list_response".into());
                self.state.terminate(error.clone(), true).await;
                Err(error)
            }
        }
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpToolResult, McpClientError> {
        let result = self
            .request(
                "tools/call",
                json!({"name": name, "arguments": arguments}),
                self.state.limits.request_timeout,
            )
            .await?;
        let result: McpToolResult = match serde_json::from_value(result) {
            Ok(result) => result,
            Err(_) => {
                let error = McpClientError::Protocol("invalid_tool_call_response".into());
                self.state.terminate(error.clone(), true).await;
                return Err(error);
            }
        };
        if !validate_content_blocks(&result.content) {
            let error = McpClientError::Protocol("invalid_tool_content".into());
            self.state.terminate(error.clone(), true).await;
            return Err(error);
        }
        Ok(result)
    }

    async fn close(&self) -> CloseResult {
        if self
            .state
            .close_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
        {
            let state = self.state.clone();
            tokio::spawn(async move {
                let result = state.close_process().await;
                state.publish_close(result);
            });
        }
        self.state.wait_for_close().await
    }
}

struct PendingGuard {
    state: Arc<StdioState>,
    id: u64,
    armed: bool,
}

impl PendingGuard {
    fn new(state: Arc<StdioState>, id: u64) -> Self {
        Self {
            state,
            id,
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let overflow = self
            .state
            .requests
            .lock()
            .map_err(|_| McpClientError::Disconnected)
            .and_then(|mut requests| {
                if requests.pending.remove(&self.id).is_some() {
                    requests.canceled.insert(self.id)
                } else {
                    Ok(())
                }
            });
        if let Err(error) = overflow {
            self.state.terminal.store(true, Ordering::Release);
            self.state.fail_pending(error.clone());
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let state = self.state.clone();
                handle.spawn(async move {
                    state.terminate(error, true).await;
                });
            }
        }
    }
}

enum Completion {
    Delivered,
    Stale,
    Unknown,
}

async fn reader_loop<R>(state: Weak<StdioState>, mut reader: R)
where
    R: tokio::io::AsyncRead + Unpin,
{
    let Some(limit) = state
        .upgrade()
        .map(|state| state.limits.inbound_frame_bytes)
    else {
        return;
    };
    let Ok(mut buffer) = FrameBuffer::new(limit) else {
        if let Some(state) = state.upgrade() {
            state
                .terminate(
                    McpClientError::Protocol("frame_buffer_allocation_failed".into()),
                    false,
                )
                .await;
        }
        return;
    };
    loop {
        let frame = match read_frame(&mut reader, &mut buffer).await {
            Ok(frame) => frame,
            Err(error) => {
                if let Some(state) = state.upgrade()
                    && !state.terminal.load(Ordering::Acquire)
                {
                    state.terminate(error, false).await;
                }
                return;
            }
        };
        let Some(state) = state.upgrade() else {
            return;
        };
        let completion = match decode_incoming(&frame) {
            Ok(IncomingMessage::Notification) => continue,
            Ok(IncomingMessage::Response { id, result }) => {
                complete_pending(&state, id, Ok(result))
            }
            Ok(IncomingMessage::Error { id, code }) => complete_pending(
                &state,
                id,
                Err(McpClientError::Protocol(format!("remote_error:{code}"))),
            ),
            Ok(IncomingMessage::Request { id, method }) => {
                if let Err(error) = respond_to_server_request(&state, id, &method).await {
                    state.terminate(error, false).await;
                    return;
                }
                continue;
            }
            Err(error) => {
                state.terminate(error, false).await;
                return;
            }
        };
        match completion {
            Completion::Delivered | Completion::Stale => {}
            Completion::Unknown => {
                state
                    .terminate(
                        McpClientError::Protocol("unknown_response_id".into()),
                        false,
                    )
                    .await;
                return;
            }
        }
    }
}

fn complete_pending(
    state: &StdioState,
    id: u64,
    result: Result<Value, McpClientError>,
) -> Completion {
    let Ok(mut requests) = state.requests.lock() else {
        return Completion::Unknown;
    };
    if let Some(sender) = requests.pending.remove(&id) {
        let _ = sender.send(result);
        Completion::Delivered
    } else if requests.canceled.remove(id) {
        Completion::Stale
    } else {
        Completion::Unknown
    }
}

async fn respond_to_server_request(
    state: &StdioState,
    id: JsonRpcId,
    method: &str,
) -> Result<(), McpClientError> {
    let payload = if method == "ping" {
        crate::toolkits::mcp::protocol::encode_success_response(
            &id,
            state.limits.outbound_frame_bytes,
        )?
    } else {
        crate::toolkits::mcp::protocol::encode_error_response(
            &id,
            -32601,
            "method not found",
            state.limits.outbound_frame_bytes,
        )?
    };
    let deadline = tokio::time::Instant::now() + state.limits.request_timeout;
    let mut writer = lock_before(&state.writer, deadline).await?;
    let writer = writer.as_mut().ok_or(McpClientError::Disconnected)?;
    tokio::time::timeout_at(
        deadline,
        write_frame(writer, &payload, state.limits.outbound_frame_bytes),
    )
    .await
    .map_err(|_| McpClientError::Timeout)?
}

fn next_request_id(next_id: &AtomicU64) -> Result<u64, McpClientError> {
    next_id
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
            current.checked_add(1)
        })
        .map_err(|_| McpClientError::RequestIdExhausted)
}

fn insert_pending(
    requests: &StdMutex<RequestRegistry>,
    id: u64,
    sender: PendingSender,
) -> Result<(), McpClientError> {
    let mut requests = requests.lock().map_err(|_| McpClientError::Disconnected)?;
    match requests.pending.entry(id) {
        Entry::Vacant(entry) => {
            entry.insert(sender);
            Ok(())
        }
        Entry::Occupied(_) => Err(McpClientError::RequestIdCollision),
    }
}

fn valid_initialize_response(value: &Value) -> bool {
    value.get("protocolVersion").and_then(Value::as_str) == Some(SUPPORTED_PROTOCOL_VERSION)
        && value
            .get("capabilities")
            .and_then(Value::as_object)
            .is_some_and(|capabilities| capabilities.get("tools").is_some_and(Value::is_object))
        && value.get("serverInfo").is_some_and(|server_info| {
            server_info.get("name").is_some_and(Value::is_string)
                && server_info.get("version").is_some_and(Value::is_string)
        })
}

fn require_canonical_file(path: &Path, code: &str) -> Result<PathBuf, McpClientError> {
    if !path.is_absolute() {
        return Err(McpClientError::Launch(code.into()));
    }
    let canonical = std::fs::canonicalize(path).map_err(|_| McpClientError::Launch(code.into()))?;
    if canonical != path || !canonical.is_file() {
        return Err(McpClientError::Launch(code.into()));
    }
    Ok(canonical)
}

fn require_canonical_directory(path: &Path) -> Result<PathBuf, McpClientError> {
    if !path.is_absolute() {
        return Err(McpClientError::Launch("invalid_working_directory".into()));
    }
    let canonical = std::fs::canonicalize(path)
        .map_err(|_| McpClientError::Launch("invalid_working_directory".into()))?;
    if canonical != path || !canonical.is_dir() {
        return Err(McpClientError::Launch("invalid_working_directory".into()));
    }
    Ok(canonical)
}

#[cfg(unix)]
fn contain_process(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.as_std_mut().process_group(0);
}

#[cfg(not(unix))]
fn contain_process(_command: &mut Command) {}

#[cfg(unix)]
fn kill_process_group(process_group_id: Option<u32>) {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::Pid,
    };

    if let Some(process_group_id) = process_group_id.and_then(|id| i32::try_from(id).ok()) {
        let _ = killpg(Pid::from_raw(process_group_id), Signal::SIGKILL);
    }
}

#[cfg(not(unix))]
fn kill_process_group(_process_group_id: Option<u32>) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_request_id_exhaustion_and_pending_collision() {
        let next = AtomicU64::new(u64::MAX);
        assert_eq!(
            next_request_id(&next),
            Err(McpClientError::RequestIdExhausted)
        );

        let pending = StdMutex::new(RequestRegistry::new(8));
        let (first, _) = oneshot::channel();
        insert_pending(&pending, 7, first).unwrap();
        let (second, _) = oneshot::channel();
        assert_eq!(
            insert_pending(&pending, 7, second),
            Err(McpClientError::RequestIdCollision)
        );
    }

    #[test]
    fn taking_process_group_disarms_drop_cleanup() {
        let process_group = ProcessGroup::new(Some(42));
        assert_eq!(process_group.take(), Some(42));
        assert_eq!(process_group.take(), None);
    }

    #[tokio::test]
    async fn close_lock_acquisition_obeys_deadline() {
        let lock = Mutex::new(());
        let _guard = lock.lock().await;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(1);
        assert!(matches!(
            lock_before(&lock, deadline).await,
            Err(McpClientError::Timeout)
        ));
    }

    #[tokio::test]
    async fn concurrent_close_callers_share_and_retry_terminal_failure() {
        let limits = McpStdioLimits {
            close_timeout: std::time::Duration::from_millis(1),
            ..McpStdioLimits::default()
        };
        let state = Arc::new(StdioState {
            writer: Mutex::new(None),
            child: Mutex::new(None),
            process_group: ProcessGroup::new(None),
            requests: StdMutex::new(RequestRegistry::new(limits.cancelled_request_ids)),
            semaphore: Arc::new(Semaphore::new(limits.max_in_flight)),
            next_id: AtomicU64::new(1),
            terminal: AtomicBool::new(false),
            limits,
            reader_task: StdMutex::new(None),
            close_started: AtomicBool::new(false),
            close_result: StdMutex::new(None),
            close_notify: Notify::new(),
        });
        let held = state.writer.lock().await;
        let first = StdioMcpClient {
            state: state.clone(),
        };
        let second = first.clone();
        let first_close = tokio::spawn(async move { first.close().await });
        let second_close = tokio::spawn(async move { second.close().await });
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        drop(held);
        let first_result = first_close.await.unwrap();
        let second_result = second_close.await.unwrap();
        assert_eq!(first_result, Err(McpClientError::Timeout));
        assert_eq!(second_result, first_result);
        let retry = StdioMcpClient { state }.close().await;
        assert_eq!(retry, first_result);
    }

    #[test]
    fn cancelled_ids_never_evict_and_fail_at_the_configured_bound() {
        let mut canceled = CanceledIds::new(2);
        canceled.insert(1).unwrap();
        canceled.insert(2).unwrap();
        assert_eq!(
            canceled.insert(3),
            Err(McpClientError::CancellationLimitExceeded { limit: 2 })
        );
        assert!(canceled.remove(1));
        assert!(!canceled.remove(1));
        canceled.insert(3).unwrap();
    }
}
