---
status: open
---

# Agent Governance -- Design Document

**Date**: 2026-06-04
**Status**: Approved; implementation open

## Goal

Add a middleware system and governance layer to langchainx so framework users
can enforce safety policies, audit tool calls, filter content, and control
agent behavior -- without modifying agent or tool implementations.

## Architecture

### Middleware Trait (`langchainx-agent`)

The trait lives in the agent crate (not core) because `before_plan` and
`after_plan` depend on `PromptArgs` (from `langchainx-prompt`) and
`AgentEvent` (from `langchainx-core`). Placing it in core would create
an inverted dependency on prompt types.

```rust
#[async_trait]
pub trait Middleware: Send + Sync {
    async fn before_tool_call(&self, ctx: ToolCallContext<'_>)
        -> Result<String, ToolError> {
        Ok(ctx.input.to_string())
    }

    async fn after_tool_call(&self, ctx: ToolCallContext<'_>,
        result: Result<String, ToolError>)
        -> Result<String, ToolError> {
        result
    }

    async fn before_plan(&self, inputs: PromptArgs)
        -> Result<PromptArgs, ChainError> {
        Ok(inputs)
    }

    async fn after_plan(&self, event: AgentEvent) -> AgentEvent {
        event
    }
}

pub struct ToolCallContext<'a> {
    pub tool_name: &'a str,
    pub input: &'a str,
    pub call_index: usize,
}
```

All methods default to pass-through. Middleware composes as
`Vec<Arc<dyn Middleware>>` on AgentExecutor.

**Execution order (onion model)**:
- before: mw1 -> mw2 -> mw3 -> tool
- after:  mw3 -> mw2 -> mw1

First `Err` in before-chain short-circuits. Transformed values thread
through each layer.

### GovernanceMiddleware (`langchainx-agent/src/governance/`)

Implements `Middleware`. Enforces a `GovernancePolicy`:

```rust
pub struct GovernancePolicy {
    pub allowed_tools: Option<HashSet<String>>,
    pub blocked_tools: HashSet<String>,
    pub content_filters: Vec<ContentFilter>,
    pub max_calls: Option<usize>,
    pub require_approval: HashSet<String>,
}

pub struct ContentFilter {
    pub name: String,
    pub pattern: Regex,
    pub target: FilterTarget,
}

pub enum FilterTarget { Input, Output, Both }
```

Builder pattern: `GovernancePolicy::builder().allow_tools([...]).build()`

Approval callback: `GovernanceMiddleware` takes an optional
`Arc<dyn ApprovalHandler>` with async `request_approval(tool, input) -> bool`.

### AuditMiddleware (`langchainx-agent/src/governance/`)

Implements `Middleware`. Append-only JSONL audit log:

```rust
pub struct AuditMiddleware {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
}

pub struct AuditEntry {
    pub timestamp: String,    // ISO-8601
    pub tool_name: String,
    pub input_preview: String,
    pub outcome: AuditOutcome,
    pub duration_ms: Option<u64>,
}

pub enum AuditOutcome { Allowed, Blocked(String), Error(String) }
```

Constructors: `AuditMiddleware::to_file(path)`,
`AuditMiddleware::to_writer(w)` (for testing).

Non-fatal writes (errors logged, never abort the chain).

### AgentExecutor Integration

New fields and builder methods on `AgentExecutor`:

```rust
middleware: Vec<Arc<dyn Middleware>>,

pub fn with_middleware(mut self, mw: Arc<dyn Middleware>) -> Self
pub fn with_middleware_stack(mut self, stack: Vec<Arc<dyn Middleware>>) -> Self
```

Executor loop changes:
- Before `agent.plan()`: run `before_plan` chain
- After `agent.plan()`: run `after_plan` chain
- Before `tool.call()`: run `before_tool_call` chain (transforms input)
- After `tool.call()`: run `after_tool_call` chain (transforms output)

### New ToolError Variant (`langchainx-core`)

```rust
#[error("governance blocked: {0}")]
GovernanceBlocked(String),
```

## Crates Affected

| Crate | Changes |
|-------|---------|
| `langchainx-core` | Add `ToolCallContext`, `ToolError::GovernanceBlocked` |
| `langchainx-agent` | Add `Middleware` trait, `governance/` module, modify `AgentExecutor` |

## Tech Decisions

1. **Middleware trait in agent, `ToolCallContext` in core** -- the trait
   references `PromptArgs` (from `langchainx-prompt`) and `AgentEvent`,
   so it cannot live in core without creating an inverted dependency.
   `ToolCallContext` and `ToolError::GovernanceBlocked` are core types.
2. **Onion execution order** -- standard middleware pattern, predictable for
   framework users.
3. **No GovernedTool wrapper** -- middleware-only approach. Per-tool policies
   achieved by checking `ctx.tool_name` in the middleware implementation.
4. **Approval via trait** -- `ApprovalHandler` trait with async method, not
   a closure generic. Keeps GovernanceMiddleware object-safe.
5. **No new dependencies** -- `regex` already in agent crate, timestamps via
   `std::time::SystemTime`.
6. **No feature flag** -- zero new external deps, core middleware trait is
   lightweight.

## Explicitly Out of Scope

- **IntentClassifier** -- follow-up after governance + audit ship
- **Chain-level middleware** -- `Chain` trait hooks deferred to a future PR
- **Trust scoring** -- interesting but not needed for initial governance
- **Policy composition** -- single policy per middleware instance; users
  compose by stacking multiple GovernanceMiddleware in the chain
- **Streaming** -- no streaming exists in the agent layer today
