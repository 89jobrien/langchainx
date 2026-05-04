# Workspace Split: Cargo Workspace + `crates/` Migration

**Date**: 2026-05-03
**Status**: Waves 0–3 complete; Wave 4 in progress

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

| Crate                    | Source                                                  | Depends on                               |
|--------------------------|---------------------------------------------------------|------------------------------------------|
| `langchainx-types`       | `src/schemas/`                                          | `serde`, `serde_json`, `thiserror`       |
| `langchainx-core`        | `src/errors.rs`, `src/language_models/`                 | `types`, `async-trait`, `tokio`, `thiserror` |
| `langchainx-prompt`      | `src/prompt/`, `src/output_parsers/`                    | `types`, `core`, `serde`, `serde_json`, `regex` |
| `langchainx-memory`      | `src/memory/`                                           | `types`, `core`                          |
| `langchainx-llm`         | `src/llm/`                                              | `types`, `core` (feature-flagged backends) |
| `langchainx-embedding`   | `src/embedding/`                                        | `types`, `core` (feature-flagged backends) |
| `langchainx-splitters`   | `src/text_splitter/`                                    | `types`, `core`, `text-splitter`, `tiktoken-rs` |
| `langchainx-testsuite`   | `src/test_utils/`                                       | `types`, `core`, `llm` (`test-utils` feature) |
| `langchainx-chain`       | `src/chain/`                                            | `types`, `core`, `prompt`, `memory`, `llm` |
| `langchainx-vectorstore` | `src/vectorstore/`                                      | `types`, `core`, `embedding` (feature-flagged) |
| `langchainx-loaders`     | `src/document_loaders/`                                 | `types`, `core`, `splitters` (feature-flagged) |
| `langchainx-tools`       | `src/tools/`                                            | `types`, `core`, `llm`                   |
| `langchainx-router`      | `src/semantic_router/`                                  | `types`, `core`, `embedding`             |
| `langchainx-agent`       | `src/agent/`                                            | `types`, `core`, `chain`, `tools`        |
| `langchainx`             | facade `lib.rs` (re-exports)                            | all crates, `default-features = false`   |

Note: `langchainx-prompt` contains all `macro_rules!` macros (`prompt_args!`, `template_fstring!`,
`template_jinja2!`, `fmt_message!`, `fmt_template!`, `fmt_placeholder!`, `message_formatter!`).
These expand to `$crate::prompt::*` paths so they must live in `langchainx-prompt` — no separate
macros crate is needed or possible.

## Dependency DAG

```
types
└── core                   (types)
    ├── prompt             (types, core)         [includes all macro_rules! macros]
    ├── memory             (types, core)
    ├── llm                (types, core)
    ├── embedding          (types, core)
    ├── splitters          (types, core)
    ├── testsuite          (types, core, llm)    [test-utils feature only, not in facade]
    ├── chain              (types, core, prompt, memory, llm)
    ├── vectorstore        (types, core, embedding)
    ├── loaders            (types, core, splitters)
    ├── tools              (types, core, llm)
    ├── router             (types, core, embedding)
    └── agent              (types, core, chain, tools)

langchainx (facade)
└── all of the above except testsuite
```

No cycles. `chain` does not depend on `agent`, `vectorstore`, or `loaders` — those compose into
chains at the application layer.

`langchainx-testsuite` is a workspace member but is NOT re-exported from the facade. It is used
as a direct dev-dependency in crates that need test helpers, gated behind
`cfg(any(test, feature = "test-utils"))`.

## Directory Layout

```
langchainx/
  Cargo.toml                    # workspace manifest
  crates/
    langchainx-types/
      Cargo.toml
      src/
    langchainx-core/
    langchainx-prompt/
    langchainx-memory/
    langchainx-llm/
    langchainx-embedding/
    langchainx-splitters/
    langchainx-testsuite/
    langchainx-chain/
    langchainx-vectorstore/
    langchainx-loaders/
    langchainx-tools/
    langchainx-router/
    langchainx-agent/
    langchainx/                 # facade
  examples/
  tests/                        # integration / e2e tests (keep at workspace root)
```

## Feature Flags

Feature flags move to each leaf crate's `Cargo.toml`. The facade `langchainx` re-exports
everything and passes feature flags through via `dep:` syntax. Users enabling
`langchainx/postgres` transitively enable both `langchainx-chain/postgres` (SqlDatabase) and
`langchainx-vectorstore/postgres` (pgvector).

The `test-utils` feature on the facade activates `langchainx-testsuite` as an optional dep.

## Migration Strategy

1. Create `Cargo.toml` workspace manifest at root.
2. Extract `langchainx-types` first — pure data types, no async deps.
3. Extract `langchainx-core` next — traits + errors, depends on `types`.
4. Extract remaining crates bottom-up following the DAG (prompt/memory/llm/embedding/splitters/
   testsuite → chain → vectorstore/loaders/tools/router → agent → facade).
5. After each crate extraction: `cargo build --all-features` and `cargo test --all-features`
   must pass before moving to the next.
6. The facade `langchainx` crate is written last; its `lib.rs` is `pub use` re-exports only.
7. Update `examples/` imports — they should only need `langchainx::` to stay as-is if facade
   re-exports are complete.
8. Bump to `0.4.0` on publish (breaking change in crate structure, even if API is preserved).

## Out of Scope

- Changing any public API signatures.
- Moving `examples/` into the workspace (they stay at root, referencing the facade crate).
- Splitting LLM backends into separate crates — feature flags are sufficient granularity there.
- Publishing individual crates to crates.io before the full migration is complete and tested.
