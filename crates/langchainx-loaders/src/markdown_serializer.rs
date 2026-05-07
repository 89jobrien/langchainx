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
                meta.insert(
                    line.trim().to_string(),
                    serde_json::Value::String(String::new()),
                );
            }
        } else {
            rest.push(line);
        }
    }
    (meta, rest.join("\n"))
}

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
    rest.strip_prefix(' ')
        .map(|title| (hashes as u8, title.trim()))
}

/// Parse body text into a nested `Section` tree.
/// Sections before the first heading are silently dropped.
pub(crate) fn parse_sections(body: &str) -> Vec<Section> {
    struct Seg {
        level: u8,
        title: String,
        content: String,
    }

    // Phase 1: collect flat segments
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
    }
    if let Some(mut seg) = current {
        seg.content = buf.join("\n").trim().to_string();
        segments.push(seg);
    }

    // Phase 2: stack-based nesting.
    // The stack holds ancestors in order; when a new section is not a child of
    // the current top, we pop completed sections up to the right parent.
    // Children are appended in document order (no reversal needed).
    let mut root: Vec<Section> = vec![];
    let mut stack: Vec<Section> = vec![];

    for seg in segments {
        let sec = Section {
            level: seg.level,
            title: seg.title,
            content: seg.content,
            children: vec![],
        };
        // Pop stack entries that cannot be the parent of `sec`
        // (i.e. those at the same level or deeper).
        loop {
            match stack.last() {
                Some(top) if top.level >= sec.level => {
                    let popped = stack.pop().unwrap();
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(popped),
                        None => root.push(popped),
                    }
                }
                _ => break,
            }
        }
        stack.push(sec);
    }
    // Drain remaining stack in reverse-pop order (deepest first, then parents).
    // Collect indices so we can process from deepest to shallowest while
    // keeping document order within each parent's children list.
    let n = stack.len();
    for i in (0..n).rev() {
        let sec = stack.remove(i);
        match stack.last_mut() {
            Some(parent) => parent.children.push(sec),
            None => root.push(sec),
        }
    }
    root
}

impl MarkdownDocument {
    pub fn parse(src: &str) -> Result<Self, MarkdownSerializerError> {
        let (frontmatter, body) = parse_frontmatter(src);
        let sections = parse_sections(&body);
        Ok(Self {
            frontmatter,
            sections,
        })
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    #[cfg(feature = "yaml")]
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }
}

impl std::str::FromStr for MarkdownDocument {
    type Err = MarkdownSerializerError;

    fn from_str(src: &str) -> Result<Self, Self::Err> {
        Self::parse(src)
    }
}

impl TryFrom<&str> for MarkdownDocument {
    type Error = MarkdownSerializerError;

    fn try_from(src: &str) -> Result<Self, Self::Error> {
        Self::parse(src)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- types ---

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

    // --- parse_frontmatter ---

    #[test]
    fn test_parse_frontmatter_extracts_keys() {
        let src = "---\ntitle: Hello\nauthor: Alice\n---\nBody text.";
        let (meta, body) = parse_frontmatter(src);
        assert_eq!(
            meta.get("title").unwrap(),
            &serde_json::Value::String("Hello".into())
        );
        assert_eq!(
            meta.get("author").unwrap(),
            &serde_json::Value::String("Alice".into())
        );
        assert_eq!(body, "Body text.");
    }

    #[test]
    fn test_parse_frontmatter_no_frontmatter() {
        let src = "# Just a heading\n\nContent.";
        let (meta, body) = parse_frontmatter(src);
        assert!(meta.is_empty());
        assert_eq!(body, src);
    }

    // --- parse_sections ---

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

    /// A depth jump from h1 directly to h3 (no h2 in between) — h3 should
    /// still become a child of h1, not a root-level section.
    #[test]
    fn test_nested_depth_jump_h1_to_h3() {
        let body = "# Top\nTop text.\n### Skip\nSkip text.";
        let sections = parse_sections(body);
        assert_eq!(sections.len(), 1, "h3 must be nested under h1, not at root");
        assert_eq!(sections[0].title, "Top");
        assert_eq!(sections[0].children.len(), 1);
        assert_eq!(sections[0].children[0].title, "Skip");
        assert_eq!(sections[0].children[0].level, 3);
    }

    /// After a deep section, a shallower sibling at the same level as an earlier
    /// ancestor must return to the correct parent level.
    #[test]
    fn test_nested_return_to_h2_after_h3() {
        let body = "# Root\n## First\n### Deep\nDeep text.\n## Second\nSecond text.";
        let sections = parse_sections(body);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].children.len(), 2, "Root must have two h2 children");
        assert_eq!(sections[0].children[0].title, "First");
        assert_eq!(sections[0].children[0].children.len(), 1);
        assert_eq!(sections[0].children[0].children[0].title, "Deep");
        assert_eq!(sections[0].children[1].title, "Second");
        assert_eq!(sections[0].children[1].content, "Second text.");
    }

    /// Content assigned to a parent heading must not include lines that belong
    /// to its child headings.
    #[test]
    fn test_nested_content_attribution() {
        let body = "# Parent\nParent only.\n## Child\nChild only.";
        let sections = parse_sections(body);
        assert_eq!(sections[0].content, "Parent only.");
        assert_eq!(sections[0].children[0].content, "Child only.");
    }

    /// Multiple root-level h1 sections each with their own h2 children.
    #[test]
    fn test_nested_multiple_h1_each_with_h2() {
        let body = "# A\n## A1\nA1.\n# B\n## B1\nB1.";
        let sections = parse_sections(body);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].title, "A");
        assert_eq!(sections[0].children.len(), 1);
        assert_eq!(sections[0].children[0].title, "A1");
        assert_eq!(sections[1].title, "B");
        assert_eq!(sections[1].children.len(), 1);
        assert_eq!(sections[1].children[0].title, "B1");
    }

    // --- MarkdownDocument API ---

    #[test]
    fn test_from_str_full_document() {
        let src = "---\ntitle: My Doc\n---\n# Intro\nHello world.\n## Details\nMore info.";
        let doc = MarkdownDocument::parse(src).unwrap();
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
        let doc = MarkdownDocument::parse(src).unwrap();
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

    #[cfg(feature = "yaml")]
    #[test]
    fn test_to_yaml_contains_title() {
        let src = "# Hello\nContent.";
        let doc = MarkdownDocument::parse(src).unwrap();
        let yaml = doc.to_yaml().unwrap();
        assert!(yaml.contains("Hello"));
        assert!(yaml.contains("sections"));
    }
}
