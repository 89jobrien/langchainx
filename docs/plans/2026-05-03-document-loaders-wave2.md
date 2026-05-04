# Document Loaders Wave 2 — Personal Utility Tier

**Date:** 2026-05-03
**Issues:** #19, #20
**Status:** approved

## Goal

Add two loaders for personal productivity sources — Obsidian vault and Google Drive.
Both depend on wave 1 being merged first. Wave 2 is independently shippable after wave 1.

## Prerequisites

- Wave 1 merged (`MarkdownLoader` #15, `CsvLoader` already exists, `PdfLoader` already exists)

## Architecture

### Crate affected

`langchainx` (single crate). New modules under `src/document_loaders/`.

### New modules

```
src/document_loaders/
  obsidian_loader/
    mod.rs
    obsidian_loader.rs   # ObsidianLoader — #19
  google_drive_loader/
    mod.rs
    google_drive_loader.rs  # GoogleDriveLoader — #20
```

## Loader designs

### ObsidianLoader (#19)

**No new dependencies** — built on `MarkdownLoader` (#15) and `tokio::fs`.

Recursively walks a vault directory using `tokio::fs::read_dir`. Skips any path
component equal to `.obsidian`. Each `.md` file loaded via `MarkdownLoader` — frontmatter
goes to metadata, body is `page_content`. Non-markdown files ignored. `source` metadata
= absolute file path as string.

```rust
pub struct ObsidianLoader {
    vault_path: PathBuf,
}
impl ObsidianLoader {
    pub fn new(vault_path: impl Into<PathBuf>) -> Self
}
```

**No feature flag** — depends only on `MarkdownLoader` which is core.

### GoogleDriveLoader (#20)

**New dependency:** `google-drive3` or raw Drive REST via `reqwest` (evaluate at impl time;
prefer raw reqwest to avoid heavy generated client if possible).

Caller provides an OAuth2 bearer token — the library does not manage credential refresh.
Accepts a folder ID or arbitrary Drive query string. Lists matching files, dispatches by
MIME type:

| MIME type | Export / download | Parser |
|-----------|-------------------|--------|
| `application/vnd.google-apps.document` | Export as `text/plain` | inline |
| `application/vnd.google-apps.spreadsheet` | Export as `text/csv` | `CsvLoader` |
| `application/pdf` | Download bytes | `PdfLoader` (feature-flagged) |
| other | skip + `log::warn!` | — |

Metadata per document: `source` (Drive web URL), `title`, `mime_type`, `modified_time`.

```rust
pub struct GoogleDriveLoader {
    token: String,           // bearer token, caller-managed
    query: DriveQuery,       // FolderId(String) | Query(String)
    client: reqwest::Client,
}
impl GoogleDriveLoader {
    pub fn from_folder(token: impl Into<String>, folder_id: impl Into<String>) -> Self
    pub fn from_query(token: impl Into<String>, query: impl Into<String>) -> Self
    pub fn with_client(self, client: reqwest::Client) -> Self  // for testing
}
```

Feature flag: `google-drive`

Unit tests: `mockito` mocks for Drive list and export endpoints.

## New dependencies

| Crate | Version | Feature flag | Already present? |
|-------|---------|--------------|-----------------|
| (none beyond reqwest) | — | `google-drive` | reqwest: yes |

If `google-drive3` is needed for auth helpers, add behind the feature flag.

## Tech decisions

- **Caller-managed token**: the library has no opinion on credential refresh (service
  account, user OAuth, workload identity). Callers inject a ready bearer token.
- **Raw reqwest over `google-drive3`**: avoids pulling in a large generated client;
  Drive's list and export REST endpoints are simple enough to call directly.
- **PDF support gated on `pdf-extract` feature**: `GoogleDriveLoader` with PDF support
  requires `features = ["google-drive", "pdf-extract"]`; without it, PDFs are skipped.

## Out of scope

- Credential refresh / token rotation
- Google Sheets cell-level metadata (exported as CSV only)
- Shared drives (only My Drive and explicit folder queries in v1)
- Google Slides, Forms, or other workspace types
