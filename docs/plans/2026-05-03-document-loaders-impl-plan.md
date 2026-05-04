# Document Loaders — Implementation Plan

**Date:** 2026-05-03
**Design docs:** wave1.md, wave2.md
**Issues:** #15, #16, #17, #18, #19, #20

## Wave 1 — Breadth tier (no auth, minimal deps)

Loaders are independent — can be parallelised across 4 agents.

### Task 1 — MarkdownLoader (#15)

Branch: `feature/issue15-markdown-loader`

- [ ] `src/document_loaders/markdown_loader/markdown_loader.rs`
  - `MarkdownLoader<R>` struct
  - `from_path` constructor (async, opens file)
  - `new(reader)` constructor
  - `parse_frontmatter(content: &str) -> (HashMap<String, Value>, String)` — hand-rolled,
    splits on `---` delimiters, parses `key: value` lines
  - `Loader` impl: read all bytes, call `parse_frontmatter`, yield one `Document`
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/markdown_loader/mod.rs` — re-export
- [ ] `src/document_loaders/mod.rs` — add `pub mod markdown_loader`
- [ ] Unit tests (in-file, `#[cfg(test)]`):
  - no frontmatter → empty metadata, full content
  - frontmatter present → metadata populated, body only in content
  - frontmatter with missing value → key present, empty string value
  - empty file → empty document

### Task 2 — JsonLoader / JsonlLoader (#16)

Branch: `feature/issue16-json-loader`

- [ ] `src/document_loaders/json_loader/json_loader.rs`
  - `JsonLoader<R>` and `JsonlLoader<R>` structs with `content_key: Option<String>`
  - `.with_content_key` builder method on both
  - `from_path` + `new(reader)` for both
  - `JsonLoader::Loader` impl: read all, parse array, map elements to Documents
  - `JsonlLoader::Loader` impl: read lines, parse each, map to Documents
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/json_loader/mod.rs` — re-export
- [ ] `src/document_loaders/mod.rs` — add `pub mod json_loader`
- [ ] Unit tests:
  - JSON array, no content_key → full object serialised as content
  - JSON array, content_key set → field as content, rest as metadata
  - JSONL, multiple lines → multiple documents
  - invalid JSON line in JSONL → error yielded in stream

### Task 3 — SitemapLoader (#17)

Branch: `feature/issue17-sitemap-loader`

- [ ] `Cargo.toml`: add `quick-xml = { version = "~0.36", optional = true }`
- [ ] `[features]`: add `sitemap = ["dep:quick-xml"]`
- [ ] `src/document_loaders/sitemap_loader/sitemap_loader.rs`
  - `SitemapLoader` struct with `url: String`, `client: reqwest::Client`
  - `new(url)` + `with_client(client)` constructors
  - `fetch_locs(url) -> Vec<String>`: fetch XML, parse `<loc>` from `<urlset>` or
    recurse one level for `<sitemapindex>` (max depth 2)
  - `Loader` impl: call `fetch_locs`, for each URL load via `HtmlLoader`, set
    `source` metadata
  - Gate entire module on `#[cfg(feature = "sitemap")]`
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/sitemap_loader/mod.rs`
- [ ] `src/document_loaders/mod.rs` — add `#[cfg(feature = "sitemap")] pub mod sitemap_loader`
- [ ] Unit tests with `mockito`: urlset response, sitemapindex response, 404 error

### Task 4 — RssLoader (#18)

Branch: `feature/issue18-rss-loader`

- [ ] `Cargo.toml`: add `rss = { version = "~2", optional = true }`
- [ ] `[features]`: add `rss = ["dep:rss"]`
- [ ] `src/document_loaders/rss_loader/rss_loader.rs`
  - `RssLoader<R>` struct
  - `from_url(url)` async constructor (fetches bytes via reqwest)
  - `new(reader: R)` where `R: Read + Send + Sync + 'static`
  - `Loader` impl: `spawn_blocking` to parse `rss::Channel::read_from`, map items
    to Documents, metadata: title, link, pub_date, author
  - Skip items where description and content are both empty, log::warn!
  - Gate on `#[cfg(feature = "rss")]`
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/rss_loader/mod.rs`
- [ ] `src/document_loaders/mod.rs` — add `#[cfg(feature = "rss")] pub mod rss_loader`
- [ ] Unit tests: in-memory RSS fixture, metadata fields populated, empty item skipped

---

## Wave 2 — Personal utility tier

Depends on wave 1 merged. Sequential: ObsidianLoader first (no new deps), then
GoogleDriveLoader.

### Task 5 — ObsidianLoader (#19)

Branch: `feature/issue19-obsidian-loader`

Depends on: wave 1 merged (MarkdownLoader available)

- [ ] `src/document_loaders/obsidian_loader/obsidian_loader.rs`
  - `ObsidianLoader { vault_path: PathBuf }`
  - `new(vault_path: impl Into<PathBuf>)`
  - `Loader` impl: async recursive walk via `tokio::fs::read_dir`; skip `.obsidian`
    path components; for each `.md` file, create `MarkdownLoader::from_path`, call
    `load()`, yield documents with `source` metadata added
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/obsidian_loader/mod.rs`
- [ ] `src/document_loaders/mod.rs` — add `pub mod obsidian_loader`
- [ ] Unit tests: `tempfile` crate (already available or add to dev-deps), create temp
  vault with notes + `.obsidian/` dir, assert `.obsidian/` skipped, frontmatter in metadata

### Task 6 — GoogleDriveLoader (#20)

Branch: `feature/issue20-google-drive-loader`

Depends on: wave 1 merged

- [ ] `Cargo.toml`: no new runtime dep (raw reqwest); add `google-drive` feature flag
- [ ] `[features]`: add `google-drive = []`
- [ ] `src/document_loaders/google_drive_loader/google_drive_loader.rs`
  - `DriveQuery` enum: `FolderId(String)`, `Query(String)`
  - `GoogleDriveLoader { token, query, client }`
  - `from_folder` + `from_query` + `with_client` constructors
  - `list_files() -> Vec<DriveFile>`: Drive files.list REST call
  - `Loader` impl: for each file dispatch by mime_type:
    - Docs → GET export?mimeType=text/plain → Document
    - Sheets → GET export?mimeType=text/csv → CsvLoader
    - PDF → download → PdfLoader (if `pdf-extract` feature enabled, else skip)
    - other → skip + log::warn!
  - Metadata: source (webViewLink), title (name), mime_type, modified_time
  - Gate on `#[cfg(feature = "google-drive")]`
  - `load_and_split` delegates to `process_doc_stream`
- [ ] `src/document_loaders/google_drive_loader/mod.rs`
- [ ] `src/document_loaders/mod.rs` — add `#[cfg(feature = "google-drive")] pub mod google_drive_loader`
- [ ] Unit tests with `mockito`: list response, doc export, sheet export, unknown type skipped

---

## Execution order

```
Wave 1 (parallel, 4 agents):
  feature/issue15-markdown-loader   --|
  feature/issue16-json-loader       --|-> merge all to develop
  feature/issue17-sitemap-loader    --|
  feature/issue18-rss-loader        --|

Wave 2 (sequential after wave 1 merged):
  feature/issue19-obsidian-loader   -> merge
  feature/issue20-google-drive-loader -> merge
```

## Definition of done (per loader)

- [ ] `cargo check --all-features` clean
- [ ] `cargo nextest run --all-features --lib` passes (new tests green)
- [ ] `cargo fmt --all -- --check` passes
- [ ] GitHub issue closed via commit message `(closes #N)`
