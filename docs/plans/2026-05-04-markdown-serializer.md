# Markdown Serializer

**Date**: 2026-05-04
**Status**: approved

## Goal

Parse a markdown string into a structured `MarkdownDocument` object that can freely serialize
to JSON (always) and YAML (feature-gated). Frontmatter is parsed into typed metadata; the body
is parsed into a nested `Section` tree delimited by headings (`#`..`######`).

## Architecture

### Crate

`langchainx-loaders` — alongside the existing `MarkdownLoader`. New module:
`src/markdown_serializer.rs`.

### New Types

```rust
/// Top-level parsed representation of a markdown file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownDocument {
    pub frontmatter: HashMap<String, serde_json::Value>,
    pub sections: Vec<Section>,
}

/// A heading-delimited section with optional nested children.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    pub level: u8,             // 1..=6
    pub title: String,
    pub content: String,       // body text between this heading and first child/sibling heading
    pub children: Vec<Section>,
}
```

### API

```rust
impl MarkdownDocument {
    /// Parse a markdown string into a MarkdownDocument.
    pub fn from_str(src: &str) -> Result<Self, MarkdownSerializerError>;

    /// Serialize to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error>;

    /// Serialize to a YAML string. Requires `yaml` feature.
    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error>;
}

impl TryFrom<&str> for MarkdownDocument { ... }  // delegates to from_str
```

### Parser Algorithm

1. Reuse `parse_frontmatter` (already in `markdown_loader.rs`) — extract it to a shared
   `fn` in the module or a private `mod frontmatter`.
2. Walk remaining lines:
   - On a heading line (`^#{1,6} `), record level + title, push a new `Section`.
   - Non-heading lines accumulate into the current section's `content`.
3. Build nested tree via a stack: when a new heading has level <= top-of-stack, pop until
   parent level < new level, then push as child.
4. Sections before the first heading go into a synthetic level-0 preamble section (or are
   discarded — TBD at impl time based on whether preamble content exists).

### Data Flow

```
&str
  └─ parse_frontmatter()  →  HashMap<String, Value>  +  body: &str
       └─ parse_sections()  →  Vec<Section>  (nested)
            └─ MarkdownDocument { frontmatter, sections }
                 ├─ .to_json()   →  String  (serde_json, always available)
                 └─ .to_yaml()   →  String  (serde_yaml, yaml feature only)
```

## Tech Decisions

| Decision | Choice | Reason |
|---|---|---|
| Crate | `langchainx-loaders` | Markdown-specific; avoids new crate |
| JSON | `serde_json` (existing dep) | No new dep needed |
| YAML | `serde_yaml` behind `yaml` feature | Optional dep, keeps default build lean |
| Nesting | Stack-based heading parser | Simple, no recursion, O(n) |
| `TryFrom` | Yes, delegates to `from_str` | Ergonomic; zero extra logic |
| Error type | New `MarkdownSerializerError` (thiserror) | Consistent with crate error patterns |

## Out of Scope

- Writing markdown back from a `MarkdownDocument` (round-trip serialization)
- Parsing inline markdown (bold, italic, links) — content is raw strings
- File I/O — `from_str` takes `&str`; callers handle reading files
- Integration with `MarkdownLoader` — separate concern, no coupling required
