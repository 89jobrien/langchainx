# Implementation Plan: Cargo Workspace Split

**Date**: 2026-05-03
**Design doc**: `2026-05-03-workspace-split.md`
**Branch**: `feature/workspace-split`
**Gate after every step**: `cargo build --all-features && cargo test --all-features`

---

## Wave 0 — Scaffold workspace

### Step 0.1 — Create workspace manifest

Replace root `Cargo.toml` `[package]` block with a `[workspace]` manifest:

```toml
[workspace]
resolver = "2"
members = [
    "crates/*",
    "examples/vector_store_surrealdb",
]

[workspace.package]
version    = "0.4.0"
edition    = "2021"
license    = "MIT"
repository = "https://github.com/89jobrien/langchainx"

[workspace.dependencies]
# pin shared deps here once crates are extracted
```

Create `crates/` directory. Keep `src/` intact — nothing moves yet.

**Gate**: `cargo build --all-features` on the existing `src/` still passes (workspace just
wraps the existing package until step 1.1).

---

## Wave 1 — Core layer (no internal deps)

### Step 1.1 — `langchainx-core`

Move:
- `src/schemas/` → `crates/langchainx-core/src/schemas/`
- `src/errors.rs` → `crates/langchainx-core/src/errors.rs`
- `src/language_models/` → `crates/langchainx-core/src/language_models/`

`crates/langchainx-core/Cargo.toml` deps: `serde`, `thiserror`, `async-trait`, `tokio`,
`serde_json`, `futures`, `async-stream`, `tokio-stream`, `secrecy`.

Update `src/lib.rs` to replace inline modules with `pub use langchainx_core::*;`.

**Gate**: build + test pass.

---

## Wave 2 — Leaf layer (depends only on core)

All four crates in this wave can be extracted in parallel (no inter-dependencies).

### Step 2.1 — `langchainx-prompt`

Move:
- `src/prompt/` → `crates/langchainx-prompt/src/prompt/`
- `src/output_parsers/` → `crates/langchainx-prompt/src/output_parsers/`

Deps: `langchainx-core`, `serde`, `serde_json`, `regex`.

### Step 2.2 — `langchainx-memory`

Move:
- `src/memory/` → `crates/langchainx-memory/src/`

Deps: `langchainx-core`.

### Step 2.3 — `langchainx-llm`

Move:
- `src/llm/` → `crates/langchainx-llm/src/`

Deps: `langchainx-core`, `async-openai`, `reqwest`, `reqwest-eventsource`,
`tiktoken-rs`, `serde`, `serde_json`. Feature-flagged: `ollama-rs`, `mistralai-client`.

Feature flags: `ollama`, `mistralai` (same as current).

### Step 2.4 — `langchainx-embedding`

Move:
- `src/embedding/` → `crates/langchainx-embedding/src/`

Deps: `langchainx-core`. Feature-flagged: `fastembed`, `ollama-rs`, `mistralai-client`,
`async-openai`.

Feature flags: `fastembed`, `ollama`, `mistralai`, `openai` (openai on by default).

**Gate after all four**: build + test pass.

---

## Wave 3 — Mid layer

### Step 3.1 — `langchainx-chain`

Move:
- `src/chain/` → `crates/langchainx-chain/src/`

Deps: `langchainx-core`, `langchainx-prompt`, `langchainx-memory`, `langchainx-llm`.
Additional: `async-trait`, `futures`, `async-stream`.

Feature flags: `postgres` (for `SqlDatabase` chain — pulls `sqlx`).

**Gate**: build + test pass.

---

## Wave 4 — Parallel leaf crates (no inter-deps beyond core/embedding)

### Step 4.1 — `langchainx-vectorstore`

Move:
- `src/vectorstore/` → `crates/langchainx-vectorstore/src/`

Deps: `langchainx-core`, `langchainx-embedding`. Feature-flagged backends: `pgvector`/`sqlx`,
`qdrant-client`, `opensearch`, `surrealdb`.

Feature flags: `postgres`, `qdrant`, `opensearch`, `sqlite-vss`, `sqlite-vec`, `surrealdb`.

### Step 4.2 — `langchainx-loaders`

Move:
- `src/document_loaders/` → `crates/langchainx-loaders/src/document_loaders/`
- `src/text_splitter/` → `crates/langchainx-loaders/src/text_splitter/`

Deps: `langchainx-core`, `csv`, `text-splitter`, `tiktoken-rs`. Feature-flagged: `lopdf`,
`pdf-extract`, `htmd`, `gix`, `flume`, `tree-sitter` family, `rss`, `quick-xml`.

Feature flags: mirror current (`lopdf`, `pdf-extract`, `html-to-markdown`, `git`,
`tree-sitter`, `rss`, `sitemap`).

### Step 4.3 — `langchainx-tools`

Move:
- `src/tools/` → `crates/langchainx-tools/src/`

Deps: `langchainx-core`, `langchainx-llm`, `reqwest`, `scraper`, `serde_json`.
Feature-flagged: `sqlx` (for sql tool).

### Step 4.4 — `langchainx-router`

Move:
- `src/semantic_router/` → `crates/langchainx-router/src/`

Deps: `langchainx-core`, `langchainx-embedding`.

**Gate after all four**: build + test pass.

---

## Wave 5 — Agent layer

### Step 5.1 — `langchainx-agent`

Move:
- `src/agent/` → `crates/langchainx-agent/src/`

Deps: `langchainx-core`, `langchainx-chain`, `langchainx-tools`, `async-trait`,
`serde`, `serde_json`.

**Gate**: build + test pass.

---

## Wave 6 — Facade

### Step 6.1 — `langchainx` facade crate

Create `crates/langchainx/src/lib.rs` as pure re-exports:

```rust
pub use langchainx_core as core;
pub use langchainx_prompt as prompt;
pub use langchainx_memory as memory;
pub use langchainx_llm as llm;
pub use langchainx_embedding as embedding;
pub use langchainx_chain as chain;
pub use langchainx_vectorstore as vectorstore;
pub use langchainx_loaders as loaders;
pub use langchainx_tools as tools;
pub use langchainx_router as router;
pub use langchainx_agent as agent;
```

Feature flags on the facade forward to each subcrate via `dep:` syntax.

Delete `src/` once facade compiles and all tests pass.

**Gate**: `cargo build --all-features && cargo test --all-features` from workspace root.

---

## Wave 7 — Cleanup + version bump

- Remove old `src/` directory.
- Update all `examples/` — fix any import paths broken by module restructuring.
- Update `README.md` install instructions to show both facade and per-crate usage.
- Bump all crate versions to `0.4.0` via `[workspace.package]`.
- Update `CHANGELOG.md`.
- Tag `v0.4.0`.

---

## Checkpoints summary

| Wave | Crates | Gate |
|---|---|---|
| 0 | workspace scaffold | existing build still passes |
| 1 | `core` | build + test |
| 2 | `prompt`, `memory`, `llm`, `embedding` | build + test |
| 3 | `chain` | build + test |
| 4 | `vectorstore`, `loaders`, `tools`, `router` | build + test |
| 5 | `agent` | build + test |
| 6 | facade, delete `src/` | build + test |
| 7 | cleanup, version bump, tag | CI green |

## Risks

- **`test_utils` feature**: currently a flag on the root crate — needs to land in `core` and
  be re-exported so integration tests compile.
- **`examples/vector_store_surrealdb`**: already a workspace member; its `langchainx` dep path
  must be updated to point at `crates/langchainx` after Wave 6.
- **Circular feature flags**: e.g. `langchainx/postgres` must correctly activate both
  `langchainx-chain/postgres` (SqlDatabase) and `langchainx-vectorstore/postgres` (pgvector).
  Verify after Wave 6.
- **`mockito` in dev-dependencies**: currently root-only — move to whichever crates have tests
  that use it (likely `llm` and `chain`).
