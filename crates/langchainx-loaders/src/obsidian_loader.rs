use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use async_trait::async_trait;
use futures::{Stream, stream};
use tokio::fs;

use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;

use crate::{Loader, LoaderError, markdown_serializer::parse_frontmatter, process_doc_stream};

/// Recursively loads all `.md` files from an Obsidian vault directory.
///
/// - Skips the `.obsidian/` configuration directory.
/// - Parses YAML frontmatter into `Document.metadata`.
/// - Sets `source` metadata to the absolute file path.
///
/// `Clone` is derived to allow callers to reuse a loader configuration across
/// multiple load calls without consuming the original.
///
/// **Note:** `load()` currently collects all matching files eagerly before
/// streaming. A future version will stream lazily.
#[derive(Debug, Clone)]
pub struct ObsidianLoader {
    vault_path: PathBuf,
}

impl ObsidianLoader {
    pub fn new<P: Into<PathBuf>>(vault_path: P) -> Self {
        Self {
            vault_path: vault_path.into(),
        }
    }

    async fn collect_md_files(dir: &Path) -> Result<Vec<PathBuf>, LoaderError> {
        let mut files = Vec::new();
        let mut queue: VecDeque<PathBuf> = VecDeque::new();
        queue.push_back(dir.to_path_buf());

        while let Some(current) = queue.pop_front() {
            let mut reader = fs::read_dir(&current).await.map_err(|e| {
                LoaderError::OtherError(format!("Failed to read dir {:?}: {}", current, e))
            })?;
            while let Some(entry) = reader.next_entry().await.map_err(|e| {
                LoaderError::OtherError(format!("Dir entry error: {}", e))
            })? {
                let file_type = entry.file_type().await.map_err(|e| {
                    LoaderError::OtherError(format!("Failed to get file type: {}", e))
                })?;

                if file_type.is_symlink() {
                    // Symlinks are not followed to avoid cycles and unguarded traversal.
                    continue;
                }

                let path = entry.path();

                if file_type.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name == ".obsidian" {
                        continue;
                    }
                    queue.push_back(path);
                } else if file_type.is_file()
                    && path.extension().and_then(|e| e.to_str()) == Some("md")
                {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }
}

#[async_trait]
impl Loader for ObsidianLoader {
    async fn load(
        self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let files = Self::collect_md_files(&self.vault_path).await?;
        let mut docs = Vec::new();
        for path in files {
            let content = fs::read_to_string(&path).await.map_err(|e| {
                LoaderError::OtherError(format!("Failed to read {:?}: {}", path, e))
            })?;
            let (mut meta, body) = parse_frontmatter(&content);
            let abs_path = match path.canonicalize() {
                Ok(p) => p.to_string_lossy().to_string(),
                Err(e) => {
                    eprintln!(
                        "warn: canonicalize failed for {:?}: {} — using original path",
                        path, e
                    );
                    path.to_string_lossy().to_string()
                }
            };
            meta.insert("source".to_string(), serde_json::Value::String(abs_path));
            let mut doc = Document::new(body);
            doc.metadata = meta;
            docs.push(Ok(doc));
        }
        Ok(Box::pin(stream::iter(docs)))
    }

    async fn load_and_split<TS: TextSplitter + 'static>(
        self,
        splitter: TS,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let doc_stream = self.load().await?;
        let stream = process_doc_stream(doc_stream, splitter).await;
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
mod tests {
    use futures_util::StreamExt;
    use tempfile::TempDir;
    use tokio::fs;

    use super::*;

    async fn make_vault() -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Regular note with frontmatter
        fs::write(
            root.join("note1.md"),
            "---\ntitle: Note One\ntags: rust\n---\n# Note One\n\nContent here.",
        )
        .await
        .unwrap();

        // Note without frontmatter
        fs::write(root.join("note2.md"), "# Plain Note\n\nNo frontmatter.")
            .await
            .unwrap();

        // Nested note
        let sub = root.join("subdir");
        fs::create_dir(&sub).await.unwrap();
        fs::write(
            sub.join("nested.md"),
            "---\nauthor: Alice\n---\nNested content.",
        )
        .await
        .unwrap();

        // .obsidian dir — should be skipped
        let obsidian = root.join(".obsidian");
        fs::create_dir(&obsidian).await.unwrap();
        fs::write(
            obsidian.join("config.md"),
            "---\ninternal: true\n---\nShould be skipped.",
        )
        .await
        .unwrap();

        dir
    }

    #[tokio::test]
    async fn test_loads_md_files_skipping_obsidian_dir() {
        let vault = make_vault().await;
        let loader = ObsidianLoader::new(vault.path());
        let mut stream = loader.load().await.unwrap();

        let mut docs: Vec<Document> = Vec::new();
        while let Some(result) = stream.next().await {
            docs.push(result.unwrap());
        }

        // Should find 3 notes (note1, note2, nested) — not the .obsidian one
        let sources: Vec<_> = docs.iter().map(|d| d.metadata.get("source")).collect();
        assert_eq!(
            docs.len(),
            3,
            "Expected 3 docs, got {}: {:?}",
            docs.len(),
            sources,
        );
    }

    #[tokio::test]
    async fn test_frontmatter_in_metadata_and_source_set() {
        let vault = make_vault().await;
        let loader = ObsidianLoader::new(vault.path());
        let mut stream = loader.load().await.unwrap();

        let mut docs: Vec<Document> = Vec::new();
        while let Some(result) = stream.next().await {
            docs.push(result.unwrap());
        }

        // All docs must have `source`
        for doc in &docs {
            assert!(
                doc.metadata.contains_key("source"),
                "Missing source in {:?}",
                doc.metadata
            );
        }

        // Find note1
        let note1 = docs
            .iter()
            .find(|d| {
                d.metadata
                    .get("source")
                    .and_then(|v| v.as_str())
                    .map(|s| s.ends_with("note1.md"))
                    .unwrap_or(false)
            })
            .expect("note1.md not found");

        assert_eq!(
            note1.metadata.get("title"),
            Some(&serde_json::Value::String("Note One".into()))
        );
        assert_eq!(note1.page_content, "# Note One\n\nContent here.");
    }

    #[tokio::test]
    async fn test_note_without_frontmatter_has_only_source_metadata() {
        let vault = make_vault().await;
        let loader = ObsidianLoader::new(vault.path());
        let mut stream = loader.load().await.unwrap();

        let mut docs: Vec<Document> = Vec::new();
        while let Some(result) = stream.next().await {
            docs.push(result.unwrap());
        }

        let note2 = docs
            .iter()
            .find(|d| {
                d.metadata
                    .get("source")
                    .and_then(|v| v.as_str())
                    .map(|s| s.ends_with("note2.md"))
                    .unwrap_or(false)
            })
            .expect("note2.md not found");

        // Only `source` key — no frontmatter keys
        assert_eq!(note2.metadata.len(), 1);
        assert_eq!(note2.page_content, "# Plain Note\n\nNo frontmatter.");
    }
}
