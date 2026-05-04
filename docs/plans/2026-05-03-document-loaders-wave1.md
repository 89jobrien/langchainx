# Document Loaders Wave 1 — Breadth Tier

**Date:** 2026-05-03
**Issues:** #15, #16, #17, #18
**Status:** approved

## Goal

Add four document loaders that require no authentication and minimal new dependencies,
maximising crates.io appeal. All implement the existing `Loader` trait and follow the
established `from_path` / `new(reader)` constructor pattern.

## Architecture

### Crate affected

`langchainx` (single crate). All loaders live under `src/document_loaders/<name>/`.

### New modules

```
src/document_loaders/
  markdown_loader/
    mod.rs
    markdown_loader.rs   # MarkdownLoader — #15
  json_loader/
    mod.rs
    json_loader.rs       # JsonLoader + JsonlLoader — #16
  sitemap_loader/
    mod.rs
    sitemap_loader.rs    # SitemapLoader — #17
  rss_loader/
    mod.rs
    rss_loader.rs        # RssLoader — #18
```

### Trait contract (unchanged)

```rust
#[async_trait]
pub trait Loader: Send + Sync {
    async fn load(self) -> Result<Pin<Box<dyn Stream<Item = Result<Document, LoaderError>>
        + Send + 'static>>, LoaderError>;
    async fn load_and_split<TS: TextSplitter + 'static>(self, splitter: TS)
        -> Result<Pin<Box<dyn Stream<Item = Result<Document, LoaderError>>
        + Send + 'static>>, LoaderError>;
}
```

`load_and_split` in all loaders delegates to the existing `process_doc_stream` helper.

### Data flow

```
Source (file / URL / reader)
  -> Loader::load()
    -> parse content
    -> extract metadata
    -> yield Document { page_content, metadata }
  -> (optional) load_and_split -> TextSplitter -> smaller Documents
```

## Loader designs

### MarkdownLoader (#15)

**No new dependencies.**

Frontmatter detection: if the file starts with `---`, split at the second `---` delimiter.
Parse frontmatter lines as `key: value` pairs using a hand-rolled parser (avoids `serde_yaml`
dep). Values stored as `serde_json::Value::String` in `Document.metadata`.

```rust
pub struct MarkdownLoader<R> { input: R }

impl MarkdownLoader<BufReader<File>> {
    pub async fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, LoaderError>
}
impl<R: AsyncRead + ...> MarkdownLoader<R> {
    pub fn new(input: R) -> Self
}
```

### JsonLoader / JsonlLoader (#16)

**No new dependencies** (`serde_json` already present).

`JsonLoader`: reads entire input, deserialises as `Value::Array`. Each element serialised
back to string as `page_content` unless `content_key` is set, in which case that field's
string value is used and remaining fields go to metadata.

`JsonlLoader`: streams line by line; each non-empty line parsed independently.

```rust
pub struct JsonLoader<R>  { input: R, content_key: Option<String> }
pub struct JsonlLoader<R> { input: R, content_key: Option<String> }
```

Both expose `.with_content_key(key: impl Into<String>) -> Self`.

### SitemapLoader (#17)

**New dependency:** `quick-xml` behind `sitemap` feature flag.

Fetches URL with `reqwest` (already present). Parses `<loc>` elements from
`<urlset>` or `<sitemapindex>` (recursive, max depth 2). Each URL fetched as HTML
and passed through the existing `HtmlLoader`. `source` URL stored in metadata.

```rust
pub struct SitemapLoader {
    url: String,
    client: reqwest::Client,
}
impl SitemapLoader {
    pub fn new(url: impl Into<String>) -> Self
    pub fn with_client(self, client: reqwest::Client) -> Self  // for testing
}
```

Feature flag: `sitemap = ["quick-xml"]`

### RssLoader (#18)

**New dependency:** `rss` crate behind `rss` feature flag.

Fetches or reads feed bytes, parses with `rss::Channel::read_from`. Each `Item` ->
`Document`: `page_content` = `description` or `content:encoded`, metadata = `title`,
`link`, `pub_date`, `author`. Malformed items skipped with `log::warn!`.

```rust
pub struct RssLoader<R> { input: R }
impl RssLoader<reqwest::Response> {
    pub async fn from_url(url: impl Into<String>) -> Result<Self, LoaderError>
}
impl<R: Read + Send + Sync + 'static> RssLoader<R> {
    pub fn new(input: R) -> Self
}
```

Feature flag: `rss = ["dep:rss"]`

## New dependencies

| Crate     | Version | Feature flag | Already present? |
|-----------|---------|--------------|-----------------|
| `quick-xml` | `~0.36` | `sitemap`  | No |
| `rss`       | `~2`    | `rss`      | No |

## Tech decisions

- **Hand-rolled frontmatter parser** over `serde_yaml`: avoids a heavy dep for a simple
  `key: value` split. Does not support nested YAML — acceptable for v1.
- **`quick-xml` not `roxmltree`**: smaller, streaming-capable, widely used.
- **`reqwest::Client` injectable in `SitemapLoader`**: allows `mockito`-based unit tests
  without live HTTP.
- **`rss` uses sync `Read`**: the `rss` crate parses from `Read`, not `AsyncRead`. Wrap
  in `tokio::task::spawn_blocking` inside `load()`.

## Out of scope

- Authenticated sources (wave 2)
- Sitemap recursion depth > 2
- Atom feed support in `RssLoader` v1
- Nested YAML frontmatter values (arrays, objects) — stored as raw strings in v1
