# Implementation Plan: Cargo Workspace Split

**Date**: 2026-05-03
**Design doc**: `2026-05-03-workspace-split.md`
**Branch**: `feature/workspace-split`
**Gate after every step**: `cargo build --all-features && cargo test --all-features`

---

## Status

| Wave | Status    | Crates                                                        |
|------|-----------|---------------------------------------------------------------|
| 0    | DONE      | workspace scaffold                                            |
| 1    | DONE      | `langchainx-core`                                             |
| 2    | DONE      | `langchainx-prompt`, `memory`, `llm`, `embedding`, `output-parsers` |
| 3    | DONE      | `langchainx-chain`                                            |
| 4    | PENDING   | `vectorstore`, `loaders`, `tools`, `router`, `splitters`, `testsuite` |
| 5    | PENDING   | `langchainx-agent`                                            |
| 6    | PENDING   | facade, delete `src/`                                         |
| 7    | PENDING   | cleanup, version bump, tag v0.4.0                             |

Note: `langchainx-types` was not in the original plan — it must be extracted before Wave 4.
See Wave 1.5 below.

---

## Wave 0 — Scaffold workspace ✓

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

**Gate**: `cargo build --all-features` on the existing `src/` still passes.

---

## Wave 1 — Core layer ✓

### Step 1.1 — `langchainx-core` ✓

Move:
- `src/schemas/` → `crates/langchainx-core/src/schemas/`
- `src/errors.rs` → `crates/langchainx-core/src/errors.rs`
- `src/language_models/` → `crates/langchainx-core/src/language_models/`

`crates/langchainx-core/Cargo.toml` deps: `serde`, `thiserror`, `async-trait`, `tokio`,
`serde_json`, `futures`, `async-stream`, `tokio-stream`, `secrecy`.

Update `src/lib.rs` to replace inline modules with `pub use langchainx_core::*;`.

**Gate**: build + test pass.

---

## Wave 1.5 — Types split

### Step 1.5.1 — `langchainx-types`

Refactor `langchainx-core` to split out pure data types:

Move from `crates/langchainx-core/src/`:
- `schemas/` → `crates/langchainx-types/src/schemas/`

`crates/langchainx-types/Cargo.toml` deps: `serde`, `serde_json`, `thiserror`. No `async-trait`,
no `tokio` — this crate must be sync-only.

Update `langchainx-core` to depend on `langchainx-types` and re-export `langchainx_types::*`.
Update all other crates that import schemas to use `langchainx-types` directly rather than
routing through `langchainx-core`.

**Gate**: build + test pass.

---

## Wave 2 — Leaf layer ✓

All crates in this wave can be extracted in parallel (no inter-dependencies among them).

### Step 2.1 — `langchainx-prompt` ✓

Move:
- `src/prompt/` → `crates/langchainx-prompt/src/prompt/`
- `src/output_parsers/` → `crates/langchainx-prompt/src/output_parsers/`

Deps: `langchainx-types`, `langchainx-core`, `serde`, `serde_json`, `regex`.

Note: all `macro_rules!` macros (`prompt_args!`, `template_fstring!`, `template_jinja2!`,
`fmt_message!`, `fmt_template!`, `fmt_placeholder!`, `message_formatter!`) stay in this crate —
they expand to `$crate::prompt::*` paths and cannot be split into a separate macros crate.

### Step 2.2 — `langchainx-memory` ✓

Move:
- `src/memory/` → `crates/langchainx-memory/src/`

Deps: `langchainx-types`, `langchainx-core`.

### Step 2.3 — `langchainx-llm` ✓

Move:
- `src/llm/` → `crates/langchainx-llm/src/`

Deps: `langchainx-types`, `langchainx-core`, `async-openai`, `reqwest`, `reqwest-eventsource`,
`tiktoken-rs`, `serde`, `serde_json`. Feature-flagged: `ollama-rs`, `mistralai-client`.

Feature flags: `ollama`, `mistralai` (same as current).

### Step 2.4 — `langchainx-embedding` ✓

Move:
- `src/embedding/` → `crates/langchainx-embedding/src/`

Deps: `langchainx-types`, `langchainx-core`. Feature-flagged: `fastembed`, `ollama-rs`,
`mistralai-client`, `async-openai`.

Feature flags: `fastembed`, `ollama`, `mistralai`, `openai` (openai on by default).

### Step 2.5 — `langchainx-splitters`

Move:
- `src/text_splitter/` → `crates/langchainx-splitters/src/`

Deps: `langchainx-types`, `langchainx-core`, `text-splitter`, `tiktoken-rs`.

### Step 2.6 — `langchainx-testsuite`

Move:
- `src/test_utils/` → `crates/langchainx-testsuite/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-llm`.

This crate is gated behind `cfg(any(test, feature = "test-utils"))`. It is a workspace member
but is NOT re-exported from the facade. Add as a dev-dependency in crates that need fake LLMs
or other test helpers.

**Gate after all steps**: build + test pass.

---

## Wave 3 — Mid layer ✓

### Step 3.1 — `langchainx-chain` ✓

Move:
- `src/chain/` → `crates/langchainx-chain/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-prompt`, `langchainx-memory`,
`langchainx-llm`. Additional: `async-trait`, `futures`, `async-stream`.

Feature flags: `postgres` (for `SqlDatabase` chain — pulls `sqlx`).

**Gate**: build + test pass.

---

## Wave 4 — Parallel leaf crates

All four crates can be extracted in parallel. Requires Wave 1.5, Wave 2.5, and Wave 2.6 complete.

### Step 4.1 — `langchainx-vectorstore`

Move:
- `src/vectorstore/` → `crates/langchainx-vectorstore/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-embedding`. Feature-flagged backends:
`pgvector`/`sqlx`, `qdrant-client`, `opensearch`, `surrealdb`.

Feature flags: `postgres`, `qdrant`, `opensearch`, `sqlite-vss`, `sqlite-vec`, `surrealdb`.

### Step 4.2 — `langchainx-loaders`

Move:
- `src/document_loaders/` → `crates/langchainx-loaders/src/document_loaders/`

Note: `src/text_splitter/` is NOT included here — it moved to `langchainx-splitters` in Wave 2.5.

Deps: `langchainx-types`, `langchainx-core`, `langchainx-splitters`, `csv`. Feature-flagged:
`lopdf`, `pdf-extract`, `htmd`, `gix`, `flume`, `tree-sitter` family, `rss`, `quick-xml`.

Feature flags: `lopdf`, `pdf-extract`, `html-to-markdown`, `git`, `tree-sitter`, `rss`, `sitemap`.

### Step 4.3 — `langchainx-tools`

Move:
- `src/tools/` → `crates/langchainx-tools/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-llm`, `reqwest`, `scraper`, `serde_json`.
Feature-flagged: `sqlx` (for sql tool).

### Step 4.4 — `langchainx-router`

Move:
- `src/semantic_router/` → `crates/langchainx-router/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-embedding`.

**Gate after all four**: build + test pass.

---

## Wave 5 — Agent layer

### Step 5.1 — `langchainx-agent`

Move:
- `src/agent/` → `crates/langchainx-agent/src/`

Deps: `langchainx-types`, `langchainx-core`, `langchainx-chain`, `langchainx-tools`,
`async-trait`, `serde`, `serde_json`.

**Gate**: build + test pass.

---

## Wave 6 — Facade

### Step 6.1 — `langchainx` facade crate

Create `crates/langchainx/src/lib.rs` as pure re-exports:

```rust
pub use langchainx_types as types;
pub use langchainx_core as core;
pub use langchainx_prompt as prompt;
pub use langchainx_memory as memory;
pub use langchainx_llm as llm;
pub use langchainx_embedding as embedding;
pub use langchainx_splitters as splitters;
pub use langchainx_chain as chain;
pub use langchainx_vectorstore as vectorstore;
pub use langchainx_loaders as loaders;
pub use langchainx_tools as tools;
pub use langchainx_router as router;
pub use langchainx_agent as agent;
```

Feature flags on the facade forward to each subcrate via `dep:` syntax.

`langchainx-testsuite` is NOT re-exported here. It is activated as a dev-dependency in
individual crates that need it, and optionally via `langchainx/test-utils` for downstream
integration tests.

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

| Wave | Crates                                                        | Gate                          |
|------|---------------------------------------------------------------|-------------------------------|
| 0    | workspace scaffold                                            | existing build still passes   |
| 1    | `core`                                                        | build + test                  |
| 1.5  | `types` (split from core)                                     | build + test                  |
| 2    | `prompt`, `memory`, `llm`, `embedding`, `splitters`, `testsuite` | build + test               |
| 3    | `chain`                                                       | build + test                  |
| 4    | `vectorstore`, `loaders`, `tools`, `router`                   | build + test                  |
| 5    | `agent`                                                       | build + test                  |
| 6    | facade, delete `src/`                                         | build + test                  |
| 7    | cleanup, version bump, tag                                    | CI green                      |

## Risks

- **`test_utils` feature**: resolved by `langchainx-testsuite` crate — used as dev-dependency
  in crates that need it, not re-exported from facade.
- **`langchainx-types` split**: crates currently importing `langchainx_core::schemas::*` must
  be updated to import from `langchainx_types::*` directly. This affects every crate.
- **`examples/vector_store_surrealdb`**: already a workspace member; its `langchainx` dep path
  must be updated to point at `crates/langchainx` after Wave 6.
- **Circular feature flags**: `langchainx/postgres` must activate both
  `langchainx-chain/postgres` (SqlDatabase) and `langchainx-vectorstore/postgres` (pgvector).
  Verify after Wave 6.
- **`mockito` in dev-dependencies**: currently root-only — move to whichever crates have tests
  that use it (likely `llm` and `chain`).
- **`langchainx-loaders` no longer includes `text_splitter/`**: any loader code that imports
  from `text_splitter` must be updated to import from `langchainx-splitters`.
