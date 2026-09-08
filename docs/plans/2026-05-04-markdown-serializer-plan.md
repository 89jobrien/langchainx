---
status: done
---

# Plan: Markdown Serializer

## Goal

Parse a markdown string into a structured `MarkdownDocument` (frontmatter + nested sections)
that serializes freely to JSON and optionally YAML.

## Architecture

- **Crates affected**: `langchainx-loaders`
- **New types**: `MarkdownDocument`, `Section`, `MarkdownSerializerError`
  in `crates/langchainx-loaders/src/markdown_serializer.rs`
- **Data flow**:
  `&str` → `parse_frontmatter()` → `HashMap<String, Value>` + body
  → `parse_sections()` → `Vec<Section>` (nested)
  → `MarkdownDocument` → `.to_json()` / `.to_yaml()`

## Tech Stack

- Rust edition 2021 (workspace)
- `serde`, `serde_json` — already in workspace
- `serde_yaml` — new optional dep, behind `yaml` feature flag
- `thiserror` — already a dep in `langchainx-loaders`

---

## Tasks

### Task 1: Add `serde_yaml` optional dependency and `yaml` feature

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/Cargo.toml`
**Run**: `cargo check -p langchainx-loaders --features yaml`

1. No failing test needed — this is a config-only change.

2. Implement — add to `Cargo.toml`:

   Under `[dependencies]`:
   ```toml
   serde      = { version = "1", features = ["derive"] }
   serde_yaml = { version = "0.9", optional = true }
   ```

   Under `[features]`:
   ```toml
   yaml = ["dep:serde_yaml"]
   ```

3. Verify:
   ```
   cargo check -p langchainx-loaders --features yaml  → clean
   cargo check -p langchainx-loaders                  → clean (no yaml)
   ```

4. Commit: `git commit -m "chore(loaders): add serde_yaml optional dep behind yaml feature"`

---

### Task 2: Define `MarkdownSerializerError`, `Section`, and `MarkdownDocument` types

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/markdown_serializer.rs` (new file)
**Run**: `cargo nextest run -p langchainx-loaders`

1. Write failing test:
   ```rust
   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_types_are_constructible() {
           let sec = Section {
               level: 1,
               title: "Hello".into(),
               content: "world".into(),
               children: vec![],
           };
           let doc = MarkdownDocument {
               frontmatter: std::collections::HashMap::new(),
               sections: vec![sec],
           };
           assert_eq!(doc.sections[0].level, 1);
       }
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders -- test_types_are_constructible`
   Expected: FAIL (file doesn't exist yet)

2. Implement — create `crates/langchainx-loaders/src/markdown_serializer.rs`:

   ```rust
   use std::collections::HashMap;

   use serde::{Deserialize, Serialize};
   use thiserror::Error;

   #[derive(Debug, Error)]
   pub enum MarkdownSerializerError {
       #[error("JSON serialization error: {0}")]
       Json(#[from] serde_json::Error),

       #[cfg(feature = "yaml")]
       #[error("YAML serialization error: {0}")]
       Yaml(#[from] serde_yaml::Error),
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Section {
       pub level: u8,
       pub title: String,
       pub content: String,
       pub children: Vec<Section>,
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct MarkdownDocument {
       pub frontmatter: HashMap<String, serde_json::Value>,
       pub sections: Vec<Section>,
   }

   #[cfg(test)]
   mod tests {
       use super::*;

       #[test]
       fn test_types_are_constructible() {
           let sec = Section {
               level: 1,
               title: "Hello".into(),
               content: "world".into(),
               children: vec![],
           };
           let doc = MarkdownDocument {
               frontmatter: HashMap::new(),
               sections: vec![sec],
           };
           assert_eq!(doc.sections[0].level, 1);
       }
   }
   ```

3. Register in `crates/langchainx-loaders/src/lib.rs` — add after `mod markdown_loader;`:
   ```rust
   mod markdown_serializer;
   pub use markdown_serializer::*;
   ```

4. Verify:
   ```
   cargo nextest run -p langchainx-loaders -- test_types_are_constructible  → green
   cargo clippy -p langchainx-loaders -- -D warnings                        → zero warnings
   ```

5. Commit: `git commit -m "feat(loaders): add MarkdownDocument and Section types"`

---

### Task 3: Extract `parse_frontmatter` to shared location

**Crate**: `langchainx-loaders`
**File(s)**:
- `crates/langchainx-loaders/src/markdown_serializer.rs`
- `crates/langchainx-loaders/src/markdown_loader.rs`
**Run**: `cargo nextest run -p langchainx-loaders`

1. Write failing test (in `markdown_serializer.rs`):
   ```rust
   #[test]
   fn test_parse_frontmatter_extracts_keys() {
       let src = "---\ntitle: Hello\nauthor: Alice\n---\nBody text.";
       let (meta, body) = parse_frontmatter(src);
       assert_eq!(meta.get("title").unwrap(), &serde_json::Value::String("Hello".into()));
       assert_eq!(meta.get("author").unwrap(), &serde_json::Value::String("Alice".into()));
       assert_eq!(body, "Body text.");
   }

   #[test]
   fn test_parse_frontmatter_no_frontmatter() {
       let src = "# Just a heading\n\nContent.";
       let (meta, body) = parse_frontmatter(src);
       assert!(meta.is_empty());
       assert_eq!(body, src);
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders -- test_parse_frontmatter`
   Expected: FAIL

2. Implement — add `parse_frontmatter` to `markdown_serializer.rs`:

   ```rust
   /// Splits YAML frontmatter (delimited by `---`) from body.
   /// Returns `(metadata_map, body)`.
   pub(crate) fn parse_frontmatter(content: &str) -> (HashMap<String, serde_json::Value>, String) {
       let mut lines = content.lines();
       let first = lines.next().unwrap_or("");
       if first.trim() != "---" {
           return (HashMap::new(), content.to_string());
       }
       let mut meta: HashMap<String, serde_json::Value> = HashMap::new();
       let mut rest: Vec<&str> = vec![];
       let mut in_front = true;
       for line in lines {
           if in_front {
               if line.trim() == "---" {
                   in_front = false;
               } else if let Some((k, v)) = line.split_once(':') {
                   meta.insert(
                       k.trim().to_string(),
                       serde_json::Value::String(v.trim().to_string()),
                   );
               } else {
                   meta.insert(line.trim().to_string(), serde_json::Value::String(String::new()));
               }
           } else {
               rest.push(line);
           }
       }
       (meta, rest.join("\n"))
   }
   ```

   Update `markdown_loader.rs` — replace the local `parse_frontmatter` with a call to the
   shared one. Change the function signature usage (it now returns
   `HashMap<String, serde_json::Value>` instead of `Vec<(String, String)>`):

   ```rust
   // At top of markdown_loader.rs, remove the old parse_frontmatter fn.
   // Import from sibling module:
   use crate::markdown_serializer::parse_frontmatter;

   // In MarkdownLoader::load(), replace:
   //   let (meta_pairs, body) = parse_frontmatter(&self.content);
   //   for (k, v) in meta_pairs { doc.metadata.insert(k, serde_json::Value::String(v)); }
   // With:
   let (meta, body) = parse_frontmatter(&self.content);
   let mut doc = Document::new(body);
   doc.metadata = meta;
   ```

3. Verify:
   ```
   cargo nextest run -p langchainx-loaders  → all green (including existing loader tests)
   cargo clippy -p langchainx-loaders -- -D warnings  → zero warnings
   ```

4. Commit: `git commit -m "refactor(loaders): extract parse_frontmatter to markdown_serializer"`

---

### Task 4: Implement `parse_sections` — flat heading scan

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/markdown_serializer.rs`
**Run**: `cargo nextest run -p langchainx-loaders -- parse_sections`

1. Write failing tests:
   ```rust
   #[test]
   fn test_parse_sections_single_heading() {
       let body = "# Hello\n\nSome content.";
       let sections = parse_sections(body);
       assert_eq!(sections.len(), 1);
       assert_eq!(sections[0].level, 1);
       assert_eq!(sections[0].title, "Hello");
       assert_eq!(sections[0].content.trim(), "Some content.");
       assert!(sections[0].children.is_empty());
   }

   #[test]
   fn test_parse_sections_no_heading_returns_empty() {
       let body = "Just plain text with no headings.";
       let sections = parse_sections(body);
       assert!(sections.is_empty());
   }

   #[test]
   fn test_parse_sections_multiple_flat() {
       let body = "# One\nContent one.\n# Two\nContent two.";
       let sections = parse_sections(body);
       assert_eq!(sections.len(), 2);
       assert_eq!(sections[0].title, "One");
       assert_eq!(sections[1].title, "Two");
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders -- test_parse_sections`
   Expected: FAIL

2. Implement — add to `markdown_serializer.rs`:

   ```rust
   /// Parse a heading line like `## Title` into `(level, title)`.
   /// Returns `None` if the line is not a heading.
   fn heading_level(line: &str) -> Option<(u8, &str)> {
       if !line.starts_with('#') {
           return None;
       }
       let hashes = line.bytes().take_while(|&b| b == b'#').count();
       if hashes > 6 {
           return None;
       }
       let rest = &line[hashes..];
       if rest.starts_with(' ') {
           Some((hashes as u8, rest[1..].trim()))
       } else {
           None
       }
   }

   /// Parse body text into a flat list of `(level, title, content)` tuples,
   /// then nest them into a tree.
   pub(crate) fn parse_sections(body: &str) -> Vec<Section> {
       // Phase 1: collect flat segments
       struct Seg {
           level: u8,
           title: String,
           content: String,
       }
       let mut segments: Vec<Seg> = vec![];
       let mut current: Option<Seg> = None;
       let mut buf: Vec<&str> = vec![];

       for line in body.lines() {
           if let Some((level, title)) = heading_level(line) {
               if let Some(mut seg) = current.take() {
                   seg.content = buf.join("\n").trim().to_string();
                   segments.push(seg);
                   buf.clear();
               }
               current = Some(Seg {
                   level,
                   title: title.to_string(),
                   content: String::new(),
               });
           } else if current.is_some() {
               buf.push(line);
           }
           // lines before first heading are silently dropped
       }
       if let Some(mut seg) = current {
           seg.content = buf.join("\n").trim().to_string();
           segments.push(seg);
       }

       // Phase 2: nest by level using a stack
       nest_segments(segments)
   }

   fn nest_segments(segments: Vec<impl Into<Section> + std::fmt::Debug>) -> Vec<Section> {
       // Re-define using concrete type
       struct Seg { level: u8, title: String, content: String }
       // (This fn is inlined below to avoid trait complexity)
       vec![] // placeholder — see full impl below
   }
   ```

   Replace the stub with the real nesting implementation (inline in `parse_sections`):

   ```rust
   pub(crate) fn parse_sections(body: &str) -> Vec<Section> {
       struct Seg { level: u8, title: String, content: String }

       let mut segments: Vec<Seg> = vec![];
       let mut current: Option<Seg> = None;
       let mut buf: Vec<&str> = vec![];

       for line in body.lines() {
           if let Some((level, title)) = heading_level(line) {
               if let Some(mut seg) = current.take() {
                   seg.content = buf.join("\n").trim().to_string();
                   segments.push(seg);
                   buf.clear();
               }
               current = Some(Seg { level, title: title.to_string(), content: String::new() });
           } else if current.is_some() {
               buf.push(line);
           }
       }
       if let Some(mut seg) = current {
           seg.content = buf.join("\n").trim().to_string();
           segments.push(seg);
       }

       // Stack-based nesting: stack holds (section, children_accumulator)
       // We build children bottom-up.
       let mut root: Vec<Section> = vec![];
       // Stack entries: (level, section_being_built)
       let mut stack: Vec<Section> = vec![];

       for seg in segments {
           let mut sec = Section {
               level: seg.level,
               title: seg.title,
               content: seg.content,
               children: vec![],
           };
           // Pop stack entries that are siblings or higher
           while let Some(top) = stack.last() {
               if top.level >= sec.level {
                   let popped = stack.pop().unwrap();
                   if let Some(parent) = stack.last_mut() {
                       parent.children.push(popped);
                   } else {
                       root.push(popped);
                   }
               } else {
                   break;
               }
           }
           stack.push(sec);
       }
       // Drain remaining stack
       while let Some(sec) = stack.pop() {
           if let Some(parent) = stack.last_mut() {
               parent.children.push(sec);
           } else {
               root.push(sec);
           }
       }
       // Children were pushed in reverse during drain — reverse each level
       fn fix_order(sections: &mut Vec<Section>) {
           sections.reverse();
           for s in sections.iter_mut() {
               fix_order(&mut s.children);
           }
       }
       fix_order(&mut root);
       root
   }
   ```

3. Verify:
   ```
   cargo nextest run -p langchainx-loaders -- test_parse_sections  → green
   cargo clippy -p langchainx-loaders -- -D warnings               → zero warnings
   ```

4. Commit: `git commit -m "feat(loaders): implement parse_sections with stack-based nesting"`

---

### Task 5: Test nested section tree

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/markdown_serializer.rs`
**Run**: `cargo nextest run -p langchainx-loaders -- test_nested`

1. Write failing tests:
   ```rust
   #[test]
   fn test_nested_h2_under_h1() {
       let body = "# Parent\nParent content.\n## Child\nChild content.";
       let sections = parse_sections(body);
       assert_eq!(sections.len(), 1);
       assert_eq!(sections[0].title, "Parent");
       assert_eq!(sections[0].children.len(), 1);
       assert_eq!(sections[0].children[0].title, "Child");
       assert_eq!(sections[0].children[0].content, "Child content.");
   }

   #[test]
   fn test_sibling_h2s_under_h1() {
       let body = "# Root\n## Alpha\nA.\n## Beta\nB.";
       let sections = parse_sections(body);
       assert_eq!(sections.len(), 1);
       assert_eq!(sections[0].children.len(), 2);
       assert_eq!(sections[0].children[0].title, "Alpha");
       assert_eq!(sections[0].children[1].title, "Beta");
   }

   #[test]
   fn test_deeply_nested() {
       let body = "# L1\n## L2\n### L3\nDeep.";
       let sections = parse_sections(body);
       assert_eq!(sections.len(), 1);
       let l2 = &sections[0].children;
       assert_eq!(l2.len(), 1);
       assert_eq!(l2[0].children[0].title, "L3");
       assert_eq!(l2[0].children[0].content, "Deep.");
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders -- test_nested`
   Expected: FAIL (tests don't exist yet)

2. Add tests to the `#[cfg(test)]` block in `markdown_serializer.rs` (copy from above).

3. Verify:
   ```
   cargo nextest run -p langchainx-loaders -- test_nested  → green
   ```

4. Commit: `git commit -m "test(loaders): add nested section tree tests"`

---

### Task 6: Implement `MarkdownDocument::from_str`, `to_json`, `TryFrom`

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/markdown_serializer.rs`
**Run**: `cargo nextest run -p langchainx-loaders -- test_markdown_document`

1. Write failing tests:
   ```rust
   #[test]
   fn test_from_str_full_document() {
       let src = "---\ntitle: My Doc\n---\n# Intro\nHello world.\n## Details\nMore info.";
       let doc = MarkdownDocument::from_str(src).unwrap();
       assert_eq!(
           doc.frontmatter.get("title").unwrap(),
           &serde_json::Value::String("My Doc".into())
       );
       assert_eq!(doc.sections.len(), 1);
       assert_eq!(doc.sections[0].title, "Intro");
       assert_eq!(doc.sections[0].children[0].title, "Details");
   }

   #[test]
   fn test_to_json_roundtrip() {
       let src = "# Hello\nContent.";
       let doc = MarkdownDocument::from_str(src).unwrap();
       let json = doc.to_json().unwrap();
       let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
       assert_eq!(parsed["sections"][0]["title"], "Hello");
   }

   #[test]
   fn test_try_from_str() {
       let src = "# Test\nBody.";
       let doc = MarkdownDocument::try_from(src).unwrap();
       assert_eq!(doc.sections[0].title, "Test");
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders -- test_markdown_document`
   Expected: FAIL

2. Implement — add to `markdown_serializer.rs`:

   ```rust
   impl MarkdownDocument {
       pub fn from_str(src: &str) -> Result<Self, MarkdownSerializerError> {
           let (frontmatter, body) = parse_frontmatter(src);
           let sections = parse_sections(&body);
           Ok(Self { frontmatter, sections })
       }

       pub fn to_json(&self) -> Result<String, serde_json::Error> {
           serde_json::to_string_pretty(self)
       }

       #[cfg(feature = "yaml")]
       pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
           serde_yaml::to_string(self)
       }
   }

   impl TryFrom<&str> for MarkdownDocument {
       type Error = MarkdownSerializerError;

       fn try_from(src: &str) -> Result<Self, Self::Error> {
           Self::from_str(src)
       }
   }
   ```

3. Verify:
   ```
   cargo nextest run -p langchainx-loaders -- test_markdown_document  → green
   cargo nextest run -p langchainx-loaders                            → all green
   cargo clippy -p langchainx-loaders -- -D warnings                  → zero warnings
   ```

4. Commit: `git commit -m "feat(loaders): implement MarkdownDocument::from_str, to_json, TryFrom"`

---

### Task 7: Feature-gate `to_yaml` and verify yaml feature build

**Crate**: `langchainx-loaders`
**File(s)**: `crates/langchainx-loaders/src/markdown_serializer.rs`
**Run**: `cargo nextest run -p langchainx-loaders --features yaml -- test_to_yaml`

1. Write failing test (gated on `yaml` feature):
   ```rust
   #[cfg(feature = "yaml")]
   #[test]
   fn test_to_yaml_contains_title() {
       let src = "# Hello\nContent.";
       let doc = MarkdownDocument::from_str(src).unwrap();
       let yaml = doc.to_yaml().unwrap();
       assert!(yaml.contains("Hello"));
       assert!(yaml.contains("sections"));
   }
   ```
   Run: `cargo nextest run -p langchainx-loaders --features yaml -- test_to_yaml`
   Expected: FAIL (test doesn't exist yet / to_yaml not yet confirmed working)

2. Add test to `#[cfg(test)]` block in `markdown_serializer.rs`.

3. Verify:
   ```
   cargo nextest run -p langchainx-loaders --features yaml -- test_to_yaml  → green
   cargo check -p langchainx-loaders                                         → clean (no yaml)
   cargo check -p langchainx-loaders --features yaml                         → clean
   cargo clippy -p langchainx-loaders --features yaml -- -D warnings         → zero warnings
   ```

4. Commit: `git commit -m "test(loaders): add yaml feature gate test for to_yaml"`

---

## Quality Gates (final)

Run before declaring done:

```
cargo nextest run -p langchainx-loaders                    # base feature set
cargo nextest run -p langchainx-loaders --features yaml    # yaml feature
cargo clippy -p langchainx-loaders -- -D warnings
cargo clippy -p langchainx-loaders --features yaml -- -D warnings
cargo fmt --all -- --check
```
