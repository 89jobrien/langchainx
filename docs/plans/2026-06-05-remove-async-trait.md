# Plan: Remove async_trait Where Safe

## Goal

Remove the `async-trait` proc macro from traits that have zero `dyn`
usage, replacing with native `async fn in traits` (stable since Rust
1.75, RFC 3185). Traits used as trait objects keep `#[async_trait]`
until Rust stabilizes async fn in `dyn Trait`.

## Context Map

### Trait Inventory

| Trait | File | `dyn` count | Action |
|-------|------|-------------|--------|
| `Loader` | `crates/langchainx-loaders/src/document_loader.rs` | 0 | **Remove** |
| `TextSplitter` | `crates/langchainx-text-splitter/src/splitter.rs` | 0 | **Remove** |
| `LLM` | `crates/langchainx-llm/src/language_models/llm.rs` | 14 | Keep |
| `Chain` | `crates/langchainx-chain/src/chain_trait.rs` | 11 | Keep |
| `Tool` | `crates/langchainx-core/src/tools.rs` | 31 | Keep |
| `Embedder` | `crates/langchainx-embedding/src/embedding/embedder_trait.rs` | 17 | Keep |
| `VectorStore` | `crates/langchainx-vectorstore/src/vectorstore.rs` | 3 | Keep |
| `Retriever` | `crates/langchainx-core/src/schemas/retrievers.rs` | 4 | Keep |
| `OutputParser` | `crates/langchainx-output-parsers/src/output_parser.rs` | 9 | Keep |

### Why Not Remove All?

Native `async fn` in traits cannot be called through `&dyn Trait` or
`Arc<dyn Trait>` — the compiler doesn't know the future's size. Seven
of nine traits are used as trait objects (89 sites across 52 files).
Removing `async_trait` from those would require either:

- `trait-variant` crate (forces `LocalTrait`/`Trait` naming inversion —
  invasive for a library)
- Manual erased wrappers (boilerplate per trait)

Neither is worth the churn. `Loader` and `TextSplitter` have zero `dyn`
usage, making them safe to migrate with no downstream impact.

### Files to Modify

**Task 1 — Loader** (17 files):
- `crates/langchainx-loaders/src/document_loader.rs` (trait def)
- `crates/langchainx-loaders/Cargo.toml`
- 15 impl files: `csv_loader.rs`, `git_commit_loader.rs`,
  `google_drive_loader/mod.rs`, `html_loader.rs`,
  `html_to_markdown_loader.rs`, `json_loader.rs`,
  `markdown_loader.rs`, `obsidian_loader.rs`, `pandoc_loader.rs`,
  `pdf_loader/lo_loader.rs`, `pdf_loader/pdf_extract_loader.rs`,
  `rss_loader.rs`, `sitemap_loader.rs`,
  `source_code_loader/source_code_loader.rs`, `text_loader.rs`

**Task 2 — TextSplitter** (5 files):
- `crates/langchainx-text-splitter/src/splitter.rs` (trait def)
- `crates/langchainx-text-splitter/Cargo.toml`
- 3 impl files: `markdown.rs`, `plain_text.rs`, `token.rs`

### Dependencies (no updates needed)

Consumers of `Loader` and `TextSplitter` call them through generics
(`impl Loader`, `TS: TextSplitter`), not trait objects. No downstream
files need changes.

### Test Coverage

| Test location | Covers |
|---------------|--------|
| `crates/langchainx-loaders/src/*.rs` (inline) | Each loader's `load()` |
| `crates/langchainx-text-splitter/src/*.rs` (inline) | Split behavior |
| `tests/` root | Integration tests use loaders via generics |

### Risk

- [x] Public API change — `impl` blocks lose `#[async_trait]`, callers
  using generics are unaffected
- [ ] No `dyn` usage — no trait-object compatibility risk
- [ ] No serialization or CLI output changes
- [ ] No new dependencies needed

## Tasks

### Task 1: Remove async_trait from Loader trait + all impls

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/document_loader.rs` + 13
loader impl files + `Cargo.toml`
**Run**: `cargo nextest run -p langchainx-loaders`

1. In `document_loader.rs`: remove `use async_trait::async_trait` and
   `#[async_trait]` from the `Loader` trait definition.
2. In each of the 15 loader files: remove `use async_trait::async_trait`
   and `#[async_trait]` from the `impl Loader for X` block.
3. Check if `async-trait` is still used elsewhere in the crate (e.g.
   other traits). If not, remove it from `Cargo.toml` dependencies.
4. Verify:
   ```
   cargo nextest run -p langchainx-loaders
   cargo clippy -p langchainx-loaders -- -D warnings
   ```
5. Commit:
   `refactor(loaders): remove async_trait from Loader -- native async fn`

### Task 2: Remove async_trait from TextSplitter trait + all impls

**Crate**: `langchainx-text-splitter`
**File(s)**: `crates/langchainx-text-splitter/src/splitter.rs` + 3 impl
files + `Cargo.toml`
**Run**: `cargo nextest run -p langchainx-text-splitter`

1. In `splitter.rs`: remove `use async_trait::async_trait` and
   `#[async_trait]` from the `TextSplitter` trait definition.
2. In `markdown.rs`, `plain_text.rs`, `token.rs`: remove
   `use async_trait::async_trait` and `#[async_trait]` from impl blocks.
3. Remove `async-trait` from `Cargo.toml` dependencies.
4. Verify:
   ```
   cargo nextest run -p langchainx-text-splitter
   cargo clippy -p langchainx-text-splitter -- -D warnings
   ```
5. Commit:
   `refactor(text-splitter): remove async_trait -- native async fn`

## Dependency Chain

```
t1 (Loader) ──┐
              ├── done
t2 (TextSplitter) ┘
```

Tasks are independent — can run in parallel.

## Future Work

When Rust stabilizes `async fn in dyn Trait`, revisit the remaining 7
traits (`LLM`, `Chain`, `Tool`, `Embedder`, `VectorStore`, `Retriever`,
`OutputParser`). Track via issue #3 comments — do not close the issue
until all traits are migrated.
