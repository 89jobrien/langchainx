use std::collections::HashMap;
use std::pin::Pin;

use crate::{Loader, LoaderError, process_doc_stream};
use async_trait::async_trait;
use futures::Stream;
use gix::ThreadSafeRepository;
use langchainx_core::schemas::Document;
use langchainx_text_splitter::TextSplitter;
use serde_json::Value;

#[derive(Clone)]
pub struct GitCommitLoader {
    repo: ThreadSafeRepository,
}

impl GitCommitLoader {
    pub fn new(repo: ThreadSafeRepository) -> Self {
        Self { repo }
    }

    pub fn from_path<P: AsRef<std::path::Path>>(directory: P) -> Result<Self, LoaderError> {
        let repo = ThreadSafeRepository::discover(directory)?;
        Ok(Self::new(repo))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn load_empty_git_repo_returns_error_instead_of_panicking() {
        let temp_dir = tempfile::tempdir().unwrap();
        let status = std::process::Command::new("git")
            .arg("init")
            .arg(temp_dir.path())
            .status()
            .unwrap();
        assert!(status.success());

        let loader = GitCommitLoader::from_path(temp_dir.path()).unwrap();
        let result = loader.load().await;

        match result {
            Ok(_) => panic!("expected empty git repository to return an error"),
            Err(error) => assert!(error.to_string().contains("Failed to read git HEAD")),
        }
    }
}

#[async_trait]
impl Loader for GitCommitLoader {
    async fn load(
        mut self,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<Document, LoaderError>> + Send + 'static>>,
        LoaderError,
    > {
        let repo = self.repo.to_thread_local();
        let head_id = repo
            .head_id()
            .map_err(|e| LoaderError::OtherError(format!("Failed to read git HEAD: {e}")))?
            .detach();

        // Since commits_iter can't be shared across thread safely, use channels as a workaround.
        let (tx, rx) = flume::bounded(1);

        tokio::spawn(async move {
            let commits = match repo.rev_walk(Some(head_id)).all() {
                Ok(commits) => commits,
                Err(e) => {
                    let _ = tx.send(Err(LoaderError::OtherError(format!(
                        "Failed to walk git commits: {e}"
                    ))));
                    return;
                }
            };

            let commits_iter = commits.map(|oid| {
                let oid = oid.map_err(|e| {
                    LoaderError::OtherError(format!("Failed to read git object id: {e}"))
                })?;
                let commit = oid.object().map_err(|e| {
                    LoaderError::OtherError(format!("Failed to load git commit object: {e}"))
                })?;
                let commit_id = commit.id;
                let author = commit.author().map_err(|e| {
                    LoaderError::OtherError(format!("Failed to read git commit author: {e}"))
                })?;
                let email = author.email.to_string();
                let name = author.name.to_string();
                let message = format!(
                    "{}",
                    commit
                        .message()
                        .map_err(|e| {
                            LoaderError::OtherError(format!(
                                "Failed to read git commit message: {e}"
                            ))
                        })?
                        .title
                );

                let mut document = Document::new(format!(
                    "commit {commit_id}\nAuthor: {name} <{email}>\n\n    {message}"
                ));
                let mut metadata = HashMap::new();
                metadata.insert("commit".to_string(), Value::from(commit_id.to_string()));

                document.metadata = metadata;
                Ok(document)
            });

            for document in commits_iter {
                if tx.send(document).is_err() {
                    // stream might have been dropped
                    break;
                }
            }
        });

        Ok(Box::pin(rx.into_stream()))
    }

    async fn load_and_split<TS: TextSplitter + 'static>(
        mut self,
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
