# Design: Third-Party Plugin Toolkits

## Goal

Adapt runtime OpenAPI operations and MCP server tools into bounded, mockable langchainx `Tool`
objects without coupling agents to a specific third-party service.

## Approved Approach

Add runtime-validated OpenAPI and MCP adapters under `langchainx_tools::toolkits`, using nested
OpenAPI arguments, explicit execution and egress policies, an injected MCP client, and a bounded
stdio MCP adapter.

## Context Map

### Files to Modify

| File | Change |
| --- | --- |
| `crates/langchainx-tools/Cargo.toml` | Add optional toolkit dependencies and features |
| `crates/langchainx-tools/src/lib.rs` | Export feature-gated `toolkits` |
| `crates/langchainx-tools/src/toolkits/mod.rs` | Export OpenAPI and MCP modules |
| `crates/langchainx-tools/src/toolkits/openapi/**` | Parse specs and adapt operations to tools |
| `crates/langchainx-tools/src/toolkits/mcp/**` | Discover MCP tools and implement bounded stdio |
| `Cargo.toml` | Forward toolkit features from the facade |
| `crates/langchainx-tools/tests/**` | Validate adapters with in-memory ports |

### Dependencies

- OpenAPI: optional `serde_yaml`, `yaml-rust2`, `secrecy`, `http`, `bytes`, and `futures-core`
  dependencies.
- MCP: optional Tokio process/I/O features; the stdio protocol subset is implemented locally so
  message limits apply before JSON allocation and decoding.
- `langchainx-testsuite` is a development dependency of `langchainx-tools`; the root facade also
  retains its pre-existing development dependency for facade conformance tests.
- Neither toolkit depends on the root facade.

### Reference Patterns

- `shell::coding_tools` returns `Vec<Arc<dyn Tool>>`.
- Generated tools override `parse_input` so structured JSON reaches `run` unchanged and raw
  arguments are not logged by the default parser.
- `langchainx-testsuite::contracts::tool` validates generated names, schemas, and calls.

### Risk

- OpenAPI tools may perform remote side effects; all operations are generated, but execution
  requires an explicit caller-provided policy.
- Specifications, HTTP responses, and MCP servers are untrusted inputs with strict size limits.
- MCP stdio launches caller-selected local code and therefore requires an absolute executable.
- Credential material must remain origin-bound and redacted.

## Crate Ownership

- **Owner**: `langchainx-tools` owns plugin validation, ports, and Tool adapters.
- **Facade**: root `langchainx` forwards optional features only.
- **Tests**: `langchainx-tools` uses shared testsuite contracts with in-memory ports.

## Shared Invariants

- Generated names are normalized to `snake_case`; duplicate exposed names fail construction.
- OpenAPI and MCP descriptions are always bounded library-generated summaries from method/path or
  tool name; plugin-provided descriptions, examples, and extensions are ignored.
- Tool schemas always contain an object root and object-valued `properties`.
- Untrusted descriptions, examples, and extensions never become model instructions by default.
- Request arguments, response content, URLs with queries, headers, and subprocess environment
  values are never logged.
- Toolkit errors contain bounded safe metadata only.

## OpenAPI Ports

```rust
#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn execute(
        &self,
        request: HttpRequest,
        limits: HttpLimits,
    ) -> Result<HttpResponse, OpenApiToolkitError>;
}

#[async_trait]
pub trait HttpEgressPolicy: Send + Sync {
    async fn authorize(&self, url: &Url) -> Result<AuthorizedOrigin, OpenApiToolkitError>;
}

#[async_trait]
pub trait OperationPolicy: Send + Sync {
    async fn authorize(
        &self,
        operation: &OperationContext,
    ) -> Result<(), OpenApiToolkitError>;
}
```

`PublicHttpEgressPolicy` rejects userinfo, non-HTTP schemes, unsafe ports, and every non-global IP
address, including loopback, private, link-local, multicast, unspecified, CGNAT, reserved,
documentation, cloud-metadata, and IPv4-mapped IPv6 ranges. It resolves every address and returns
an `AuthorizedOrigin` containing the exact scheme, host, port, and pinned global addresses.
Redirects are disabled. `ReqwestTransport` sends against those pinned addresses while preserving
the authorized host for HTTP and TLS.

`AllowAllOperations` and `ReadOnlyOperations` implement `OperationPolicy`. Toolkit construction
requires callers to choose a policy explicitly. This preserves the requested all-operation tool
discovery while making permission to execute mutations an explicit capability.

## OpenAPI Request Types

```rust
#[derive(Clone)]
pub struct HttpRequest {
    method: String,
    url: Url,
    headers: HeaderMap<SecretString>,
    body: Option<Vec<u8>>,
    authorized_origin: AuthorizedOrigin,
}

impl HttpRequest {
    pub fn method(&self) -> &str;
    pub fn url(&self) -> &Url;
    pub fn headers(&self) -> &HeaderMap<SecretString>;
    pub fn body(&self) -> Option<&[u8]>;
    pub fn authorized_origin(&self) -> &AuthorizedOrigin;
}

pub type HttpBody = Pin<Box<dyn Stream<Item = Result<Bytes, OpenApiToolkitError>> + Send>>;

pub struct HttpResponse {
    status: u16,
    headers: HeaderMap<HeaderValue>,
    body: HttpBody,
}

impl HttpResponse {
    pub fn new(status: u16, headers: HeaderMap<HeaderValue>, body: HttpBody) -> Self;
    pub fn status(&self) -> u16;
    pub fn headers(&self) -> &HeaderMap<HeaderValue>;
    pub fn into_body(self) -> HttpBody;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct HttpOrigin {
    scheme: String,
    host: String,
    port: u16,
}

impl HttpOrigin {
    pub fn try_from_url(url: &Url) -> Result<Self, OpenApiToolkitError>;
    pub fn scheme(&self) -> &str;
    pub fn host(&self) -> &str;
    pub fn port(&self) -> u16;
}

#[derive(Clone, Debug)]
pub struct AuthorizedOrigin {
    origin: HttpOrigin,
    addresses: Vec<IpAddr>,
}

impl AuthorizedOrigin {
    pub fn new(
        origin: HttpOrigin,
        addresses: Vec<IpAddr>,
    ) -> Result<Self, OpenApiToolkitError>;
    pub fn origin(&self) -> &HttpOrigin;
    pub fn addresses(&self) -> &[IpAddr];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HttpLimits {
    pub request_body_bytes: usize,
    pub response_body_bytes: usize,
    pub url_bytes: usize,
    pub header_bytes: usize,
    pub timeout: Duration,
}
```

`HttpRequest` has a custom `Debug` implementation that omits the body, query string, header
values, and resolved addresses. `HttpResponse` intentionally does not implement `Debug` because
its body is sensitive and streaming. `HttpLimits::default()` uses secure nonzero bounds; toolkit
construction rejects zero values or values above library ceilings.

## OpenAPI Toolkit API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenApiFormat {
    Json,
    Yaml,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenApiLimits {
    pub specification_bytes: usize,
    pub operations: usize,
    pub schema_nodes: usize,
    pub reference_depth: usize,
    pub string_bytes: usize,
    pub http: HttpLimits,
}

impl Default for OpenApiLimits {
    fn default() -> Self;
}

pub struct OpenApiToolkitBuilder {
    specification: String,
    format: OpenApiFormat,
    transport: Arc<dyn HttpTransport>,
    egress_policy: Arc<dyn HttpEgressPolicy>,
    operation_policy: Option<Arc<dyn OperationPolicy>>,
    server_override: Option<Url>,
    credential_headers: HeaderMap<SecretString>,
    credential_origin: Option<HttpOrigin>,
    name_prefix: Option<String>,
    limits: OpenApiLimits,
}

impl OpenApiToolkitBuilder {
    pub fn new(
        specification: impl Into<String>,
        format: OpenApiFormat,
        transport: Arc<dyn HttpTransport>,
        egress_policy: Arc<dyn HttpEgressPolicy>,
    ) -> Self;
    pub fn with_operation_policy(self, policy: Arc<dyn OperationPolicy>) -> Self;
    pub fn with_server_override(self, server: Url) -> Self;
    pub fn with_credential_header(
        self,
        origin: HttpOrigin,
        name: HeaderName,
        value: SecretString,
    ) -> Result<Self, OpenApiToolkitError>;
    pub fn with_name_prefix(self, prefix: impl Into<String>) -> Self;
    pub fn with_limits(self, limits: OpenApiLimits) -> Self;
    pub fn build(self) -> Result<OpenApiToolkit, OpenApiToolkitError>;
}

#[derive(Clone)]
pub struct OpenApiToolkit {
    operations: Vec<OpenApiOperation>,
    transport: Arc<dyn HttpTransport>,
    egress_policy: Arc<dyn HttpEgressPolicy>,
    operation_policy: Arc<dyn OperationPolicy>,
    credential_headers: HeaderMap<SecretString>,
    limits: OpenApiLimits,
}

impl OpenApiToolkit {
    pub fn tools(&self) -> Vec<Arc<dyn Tool>>;
}
```

`OpenApiOperation` and `OpenApiTool` remain crate-private.

## OpenAPI Validation and Mapping

- Accept OpenAPI 3.0 JSON or YAML only. Reject oversized documents before parsing.
- Detect local-reference cycles and bound reference depth, schema nodes, operations, and strings.
- Never fetch external references. Unsupported references or constructs fail toolkit construction.
- Require `operationId`; without an override, resolve operation-level servers before path-level
  and document-level servers. A builder server override replaces every declared server.
- Generate all GET, HEAD, OPTIONS, POST, PUT, PATCH, and DELETE operations.
- Use nested `path`, `query`, `headers`, `cookies`, and `body` argument objects.
- Support primitive parameters in query `form`, path/header `simple`, and cookie `form` styles.
  Support arrays for query `form` with either explode setting, path/header `simple` with
  `explode=false`, and cookie `form` with `explode=true`. Reject object parameters, matrix,
  label, deepObject, `allowReserved`, and every other combination during construction.
- Support `application/json` bodies only. Bound serialized arguments before transport execution.
- Canonicalize names and values using `HeaderName` and `HeaderValue`; reject credentials, `Host`,
  `Content-Length`, `Transfer-Encoding`, connection-specific/hop-by-hop headers, CR/LF injection,
  and case-insensitive collisions with configured headers.
- Construct paths from encoded relative segments without authority-changing URL joins, then assert
  the final origin exactly matches the authorized origin.
- Require one explicit credential origin when credentials are configured. The server override is
  applied before all operation-level server resolution. Reject any operation whose exact scheme,
  host, and port differ from the credential origin; never forward credentials across origins.
- Wrap successful output in a JSON envelope identifying it as untrusted third-party data.
- Non-success responses become bounded, redacted `ToolError::ExecutionFailed` values.
- Bound the final URL, serialized request, and cumulative headers before transport. The complete
  policy, request, transport, and body-read sequence shares one timeout. `HttpBody` yields
  post-decompression bytes, which the Tool reads incrementally and rejects at the response limit.
  `HttpTransport` implementations must reject cumulative response headers before returning;
  `ReqwestTransport` configures this limit before sending.

## OpenAPI Errors

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OpenApiToolkitError {
    InvalidSpecification(String),
    UnsupportedConstruct(String),
    MissingServer,
    MissingOperationId { method: String, path: String },
    DuplicateToolName(String),
    InvalidArguments(String),
    MissingOperationPolicy,
    MutatingOperationDenied(String),
    EgressDenied(String),
    ProtectedHeader(String),
    RequestTooLarge { limit: usize },
    ResponseTooLarge { limit: usize },
    Timeout,
    Transport(String),
}
```

## MCP Port and Types

```rust
#[async_trait]
pub trait McpClient: Send + Sync {
    async fn list_tools(
        &self,
        cursor: Option<String>,
    ) -> Result<McpToolPage, McpClientError>;
    async fn call_tool(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<McpToolResult, McpClientError>;
    async fn close(&self) -> Result<(), McpClientError>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct McpToolPage {
    pub tools: Vec<McpToolDefinition>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct McpToolResult {
    pub content: Value,
    pub is_error: bool,
}

impl Debug for McpToolResult {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result;
}
```

## MCP Toolkit API

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpToolkitLimits {
    pub tools: usize,
    pub discovery_pages: usize,
    pub discovery_bytes: usize,
    pub cursor_bytes: usize,
    pub schema_bytes: usize,
    pub argument_bytes: usize,
    pub result_bytes: usize,
    pub discovery_timeout: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct McpStdioLimits {
    pub inbound_frame_bytes: usize,
    pub outbound_frame_bytes: usize,
    pub max_in_flight: usize,
    pub request_timeout: Duration,
    pub close_timeout: Duration,
}

impl Default for McpToolkitLimits {
    fn default() -> Self;
}

impl Default for McpStdioLimits {
    fn default() -> Self;
}

#[derive(Clone)]
pub struct McpToolkit {
    client: Arc<dyn McpClient>,
    definitions: Vec<McpToolDefinition>,
    name_prefix: Option<String>,
    limits: McpToolkitLimits,
}

impl McpToolkit {
    pub async fn new(
        client: Arc<dyn McpClient>,
        limits: McpToolkitLimits,
    ) -> Result<Self, McpToolkitError>;
    pub fn with_name_prefix(self, prefix: impl Into<String>) -> Result<Self, McpToolkitError>;
    pub fn tools(&self) -> Vec<Arc<dyn Tool>>;
    pub async fn close(&self) -> Result<(), McpClientError>;
}
```

Toolkit construction follows `next_cursor` until exhaustion under one discovery timeout, rejecting
cursor loops, oversized/empty cursor chains, excessive pages, cumulative discovery bytes,
duplicate names, excessive tools, oversized schemas, and non-object input schemas. Every limits
field has a secure nonzero default and a library maximum; construction rejects invalid values.
`McpTool` remains crate-private and overrides `parse_input` to preserve structured JSON without
logging. `McpToolResult` uses a custom redacted `Debug` implementation that reports only content
size and error state.

## Bounded MCP Stdio Adapter

```rust
#[derive(Clone)]
pub struct StdioMcpConfig {
    executable: PathBuf,
    args: Vec<OsString>,
    environment: BTreeMap<OsString, SecretString>,
    working_directory: Option<PathBuf>,
    limits: McpStdioLimits,
}

impl StdioMcpConfig {
    pub fn new(executable: impl Into<PathBuf>) -> Self;
    pub fn with_arg(self, arg: impl Into<OsString>) -> Self;
    pub fn with_args<I, S>(self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>;
    pub fn with_environment(
        self,
        name: impl Into<OsString>,
        value: SecretString,
    ) -> Self;
    pub fn with_working_directory(self, path: impl Into<PathBuf>) -> Self;
    pub fn with_limits(self, limits: McpStdioLimits) -> Self;
}

pub struct StdioMcpClient {
    state: Arc<StdioState>,
}

impl StdioMcpClient {
    pub async fn connect(config: StdioMcpConfig) -> Result<Self, McpClientError>;
}
```

The adapter requires a canonical absolute executable, uses `env_clear`, adds only configured
environment entries, launches without a shell, and uses a canonical controlled working directory.
It performs the MCP initialize handshake, then supports `tools/list` and `tools/call` over
newline-delimited JSON-RPC 2.0. Frames are read incrementally into a bounded buffer before JSON
decoding. Oversized frames, malformed protocol messages, timeouts, or disconnects terminate the
child process. Complete outbound JSON-RPC frames are measured before writing. A semaphore bounds
in-flight calls; monotonically increasing request IDs and a pending-call map correlate responses.
Cancellation removes pending calls. Stderr is discarded. The child uses kill-on-drop, process-group
containment where supported, and bounded graceful shutdown followed by force-kill. `close` is
idempotent: it rejects new calls, allows already-completed responses to resolve, cancels remaining
in-flight calls at the close timeout, and makes every clone return `Disconnected` afterward.
Every stdio limit has a secure nonzero default and a library maximum; connection rejects invalid
limits before launching a process.

## MCP Errors

```rust
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpClientError {
    Launch(String),
    Protocol(String),
    Timeout,
    MessageTooLarge { limit: usize },
    Disconnected,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum McpToolkitError {
    Client(#[from] McpClientError),
    DuplicateToolName(String),
    InvalidToolSchema(String),
    TooManyTools { limit: usize },
    CursorLoop,
}
```

## Data Flow

1. OpenAPI: bounded specification -> validated operations -> nested Tool arguments -> policy and
   egress authorization -> bounded HTTP request -> untrusted-data envelope.
2. MCP: initialized client -> paginated bounded discovery -> Tool adapters -> bounded JSON-RPC
   call -> untrusted-data envelope.

## Out of Scope

- OpenAPI 3.1-only constructs, external references, non-JSON bodies, and binary result decoding.
- MCP Streamable HTTP, SSE, roots, prompts, resources, subscriptions, and server notifications.
- Credential discovery, refresh, persistence, or inherited process environment.
- Automatic approval derived from plugin metadata or plugin output.

## Compatibility

- Breaking API changes: none; all APIs are additive and feature-gated.
- Features: `openapi-toolkit`, `mcp-toolkit`, and aggregate `plugin-toolkits`.
- Generated tools satisfy the shared Tool conformance contract with only in-memory test ports.
