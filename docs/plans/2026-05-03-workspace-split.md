# Workspace Split: Cargo Workspace + `crates/` Migration

**Date**: 2026-05-03
**Status**: Design approved, not yet implemented

## Goal

Convert the single `langchainx` crate into a Cargo workspace with per-concern crates under
`crates/`. Motivations:

- **Compile times**: independent crates compile in parallel; changing one LLM backend does not
  rebuild the vectorstore or agent crates.
- **Publishing granularity**: users can depend on `langchainx-core` or `langchainx-llm` without
  pulling the full dep tree.
- **Boundary enforcement**: the type system prevents cross-layer leakage (e.g. `vectorstore`
  cannot accidentally import `agent` internals).

## Crate Map

| Crate | Source | Depends on |
|---|---|---|
| `langchainx-core` | `src/schemas/`, `src/errors.rs`, `src/language_models/` | `serde`, `thiserror`, `async-trait`, `tokio` |
| `langchainx-prompt` | `src/prompt/`, `src/output_parsers/` | `core` |
| `langchainx-memory` | `src/memory/` | `core` |
| `langchainx-llm` | `src/llm/` | `core` (feature-flagged backends) |
| `langchainx-embedding` | `src/embedding/` | `core` (feature-flagged backends) |
| `langchainx-chain` | `src/chain/` | `core`, `prompt`, `memory`, `llm` |
| `langchainx-vectorstore` | `src/vectorstore/` | `core`, `embedding` (feature-flagged backends) |
| `langchainx-loaders` | `src/document_loaders/`, `src/text_splitter/` | `core` (feature-flagged) |
| `langchainx-tools` | `src/tools/` | `core`, `llm` |
| `langchainx-router` | `src/semantic_router/` | `core`, `embedding` |
| `langchainx-agent` | `src/agent/` | `core`, `chain`, `tools` |
| `langchainx` | facade `lib.rs` (re-exports) | all crates, `default-features = false` |

## Dependency DAG

```
core
├── prompt        (core)
├── memory        (core)
├── llm           (core)
├── embedding     (core)
├── chain         (core, prompt, memory, llm)
├── vectorstore   (core, embedding)
├── loaders       (core)
├── tools         (core, llm)
├── router        (core, embedding)
└── agent         (core, chain, tools)

langchainx (facade)
└── all of the above
```

No cycles. `chain` is the unit of work — it does not depend on `agent`, `vectorstore`, or
`loaders`. Those compose _into_ chains at the application layer.

## Directory Layout

```
langchainx/
  Cargo.toml                  # workspace manifest
  crates/
    langchainx-core/
      Cargo.toml
      src/
    langchainx-prompt/
    langchainx-memory/
    langchainx-llm/
    langchainx-embedding/
    langchainx-chain/
    langchainx-vectorstore/
    langchainx-loaders/
    langchainx-tools/
    langchainx-router/
    langchainx-agent/
    langchainx/               # facade
  examples/
  tests/                      # integration / e2e tests (keep at workspace root)
```

## Feature Flags

Feature flags move to each leaf crate's `Cargo.toml`. The facade `langchainx` re-exports
everything and passes feature flags through via `dep:` syntax. Users enabling
`langchainx/postgres` transitively enable `langchainx-vectorstore/postgres`.

## Migration Strategy

1. Create `Cargo.toml` workspace manifest at root; move current `[package]` into
   `crates/langchainx-core/Cargo.toml` as the first crate.
2. Extract crates bottom-up following the DAG (core → prompt/memory/llm/embedding →
   chain → vectorstore/loaders/tools/router → agent → facade).
3. After each crate extraction: `cargo build --all-features` and `cargo test --all-features`
   must pass before moving to the next.
4. The facade `langchainx` crate is written last; its `lib.rs` is `pub use` re-exports only.
5. Update `examples/` imports — they should only need to change `langchainx::` to stay as-is
   if the facade re-exports are complete.
6. Bump to `0.4.0` on publish (breaking change in crate structure, even if API is preserved).

## Out of Scope

- Changing any public API signatures.
- Moving `examples/` into the workspace (they stay at root, referencing the facade crate).
- Splitting LLM backends into separate crates — feature flags are sufficient granularity there.
- Publishing individual crates to crates.io before the full migration is complete and tested.
