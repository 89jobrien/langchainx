//! session — stable workspace identity for GODMODE.session.json.
//!
//! Derives and persists a [`SessionIdentity`] that pegs a godmode session to:
//! - `godmode_id`: `<repo>-<owner>` from the workspace `Cargo.toml` repository URL.
//! - `project_id`: path-encoded absolute cwd.
//! - `conversation_id`: most recent UUID directory under `~/.claude/projects/<project_id>/`.
//! - `last_heartbeat`: ISO-8601 timestamp of the last heartbeat write.
//!
//! Takeover logic: if an existing session is stale (heartbeat older than `max_stale`)
//! **and** the active conversation differs, [`should_takeover`] returns `true`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Default session file name written under `.ctx/`.
pub const SESSION_FILE_NAME: &str = "GODMODE.session.json";

/// Default stale threshold for takeover detection.
pub const DEFAULT_MAX_STALE: Duration = Duration::from_secs(15 * 60);

// ── Types ─────────────────────────────────────────────────────────────────────

/// Stable workspace identity written to `GODMODE.session.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIdentity {
    /// `<repo>-<owner>` derived from the Cargo.toml `repository` URL.
    pub godmode_id: String,
    /// Path-encoded cwd: absolute path with `/` replaced by `-`.
    pub project_id: String,
    /// Most recent UUID directory found under `~/.claude/projects/<project_id>/`.
    pub conversation_id: Option<String>,
    /// Unix timestamp (seconds) of the last heartbeat write.
    pub last_heartbeat: u64,
}

// ── Derivation helpers ────────────────────────────────────────────────────────

/// Parse the `repository` field from a `Cargo.toml` file and return
/// `"<repo>-<owner>"` (e.g. `"langchainx-89jobrien"`).
///
/// # Errors
///
/// Returns an error if the file cannot be read or the `repository` key is absent
/// or does not follow `https://github.com/<owner>/<repo>` conventions.
pub fn derive_godmode_id(cargo_toml_path: &Path) -> Result<String> {
    let text = fs::read_to_string(cargo_toml_path)
        .with_context(|| format!("reading {}", cargo_toml_path.display()))?;

    let repo_url = extract_repository_from_toml(&text)
        .with_context(|| format!("no `repository` key in {}", cargo_toml_path.display()))?;

    parse_godmode_id_from_url(&repo_url)
        .with_context(|| format!("cannot parse repository URL: {repo_url}"))
}

/// Extract the raw value of the first `repository = "..."` line in a TOML string.
fn extract_repository_from_toml(toml_text: &str) -> Option<String> {
    for line in toml_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("repository") {
            // Handle `repository = "url"` and `repository  = "url"`
            if let Some(rest) = trimmed.split_once('=').map(|(_, v)| v.trim()) {
                let url = rest.trim_matches('"').trim().to_string();
                if !url.is_empty() {
                    return Some(url);
                }
            }
        }
    }
    None
}

/// Turn `https://github.com/owner/repo` → `"repo-owner"`.
fn parse_godmode_id_from_url(url: &str) -> Option<String> {
    // Strip trailing `.git` if present.
    let url = url.trim_end_matches(".git");
    let segments: Vec<&str> = url.rsplitn(3, '/').collect();
    // rsplitn gives: [repo, owner, prefix]
    if segments.len() >= 2 {
        let repo = segments[0];
        let owner = segments[1];
        if !repo.is_empty() && !owner.is_empty() {
            return Some(format!("{repo}-{owner}"));
        }
    }
    None
}

/// Encode `cwd` as a stable project identifier: absolute path with `/` → `-`.
///
/// Leading `-` (from the leading `/`) is stripped to keep the id readable.
pub fn derive_project_id(cwd: &Path) -> String {
    let raw = cwd.to_string_lossy().replace('/', "-");
    raw.trim_start_matches('-').to_string()
}

/// Scan `~/.claude/projects/<project_id>/` and return the name of the most
/// recently modified UUID-shaped subdirectory, if any.
pub fn find_conversation_id(project_id: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let projects_dir = PathBuf::from(home)
        .join(".claude")
        .join("projects")
        .join(project_id);

    let entries = fs::read_dir(&projects_dir).ok()?;

    let mut candidates: Vec<(u64, String)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // UUID v4: 8-4-4-4-12 hex chars separated by `-`
            if is_uuid_like(&name) {
                let mtime = e
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                Some((mtime, name))
            } else {
                None
            }
        })
        .collect();

    candidates.sort_by(|a, b| b.0.cmp(&a.0)); // most recent first
    candidates.into_iter().next().map(|(_, name)| name)
}

/// Lightweight UUID-shape check (8-4-4-4-12 hex groups).
fn is_uuid_like(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected = [8usize, 4, 4, 4, 12];
    parts
        .iter()
        .zip(expected.iter())
        .all(|(p, &len)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
}

// ── I/O ───────────────────────────────────────────────────────────────────────

/// Serialize `identity` to `path` as pretty-printed JSON.
///
/// The parent directory is created if it does not exist.
pub fn write_session(path: &Path, identity: &SessionIdentity) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(identity).context("serialising SessionIdentity")?;
    fs::write(path, json).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Read and deserialise a [`SessionIdentity`] from `path`.
///
/// Returns `Ok(None)` when the file does not exist.
pub fn read_session(path: &Path) -> Result<Option<SessionIdentity>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let identity =
        serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    Ok(Some(identity))
}

// ── Takeover detection ────────────────────────────────────────────────────────

/// Return `true` when `existing` should be taken over by a new session.
///
/// Takeover is triggered when **both** conditions hold:
/// 1. The heartbeat is older than `max_stale`.
/// 2. The incoming `new_conversation` differs from the stored one.
pub fn should_takeover(
    existing: &SessionIdentity,
    new_conversation: &str,
    max_stale: Duration,
) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let age = now.saturating_sub(existing.last_heartbeat);
    let is_stale = age >= max_stale.as_secs();
    let conversation_differs = existing
        .conversation_id
        .as_deref()
        .map(|c| c != new_conversation)
        .unwrap_or(true);

    is_stale && conversation_differs
}

/// Return the current Unix timestamp in seconds.
pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tempfile::tempdir;

    // ── derive_godmode_id ─────────────────────────────────────────────────────

    #[test]
    fn derive_godmode_id_parses_github_url() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
name = "langchainx"
repository = "https://github.com/89jobrien/langchainx"
"#,
        )
        .unwrap();

        let id = derive_godmode_id(&cargo_toml).unwrap();
        assert_eq!(id, "langchainx-89jobrien");
    }

    #[test]
    fn derive_godmode_id_strips_git_suffix() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(
            &cargo_toml,
            r#"[package]
repository = "https://github.com/alice/myrepo.git"
"#,
        )
        .unwrap();

        let id = derive_godmode_id(&cargo_toml).unwrap();
        assert_eq!(id, "myrepo-alice");
    }

    #[test]
    fn derive_godmode_id_errors_on_missing_repository() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"foo\"\n").unwrap();

        assert!(derive_godmode_id(&cargo_toml).is_err());
    }

    // ── derive_project_id ─────────────────────────────────────────────────────

    #[test]
    fn derive_project_id_encodes_path() {
        let cwd = Path::new("/Users/joe/dev/langchainx");
        assert_eq!(derive_project_id(cwd), "Users-joe-dev-langchainx");
    }

    #[test]
    fn derive_project_id_no_leading_dash() {
        let cwd = Path::new("/home/user/project");
        let id = derive_project_id(cwd);
        assert!(!id.starts_with('-'));
    }

    // ── should_takeover ───────────────────────────────────────────────────────

    #[test]
    fn should_takeover_true_when_stale_and_different_conversation() {
        let existing = SessionIdentity {
            godmode_id: "langchainx-89jobrien".into(),
            project_id: "Users-joe-dev-langchainx".into(),
            conversation_id: Some("old-conv-id".into()),
            // Heartbeat 30 minutes ago.
            last_heartbeat: now_unix_secs().saturating_sub(30 * 60),
        };

        assert!(should_takeover(
            &existing,
            "new-conv-id",
            Duration::from_secs(15 * 60)
        ));
    }

    #[test]
    fn should_takeover_false_when_fresh() {
        let existing = SessionIdentity {
            godmode_id: "langchainx-89jobrien".into(),
            project_id: "Users-joe-dev-langchainx".into(),
            conversation_id: Some("old-conv-id".into()),
            // Heartbeat 1 minute ago — still fresh.
            last_heartbeat: now_unix_secs().saturating_sub(60),
        };

        assert!(!should_takeover(
            &existing,
            "new-conv-id",
            Duration::from_secs(15 * 60)
        ));
    }

    #[test]
    fn should_takeover_false_when_stale_but_same_conversation() {
        let existing = SessionIdentity {
            godmode_id: "langchainx-89jobrien".into(),
            project_id: "Users-joe-dev-langchainx".into(),
            conversation_id: Some("same-conv".into()),
            last_heartbeat: now_unix_secs().saturating_sub(30 * 60),
        };

        assert!(!should_takeover(
            &existing,
            "same-conv",
            Duration::from_secs(15 * 60)
        ));
    }

    // ── write / read roundtrip ────────────────────────────────────────────────

    #[test]
    fn write_then_read_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join(".ctx").join(SESSION_FILE_NAME);

        let identity = SessionIdentity {
            godmode_id: "langchainx-89jobrien".into(),
            project_id: "Users-joe-dev-langchainx".into(),
            conversation_id: Some("abc12345-0000-0000-0000-000000000000".into()),
            last_heartbeat: 1_700_000_000,
        };

        write_session(&path, &identity).unwrap();
        let loaded = read_session(&path).unwrap().unwrap();
        assert_eq!(identity, loaded);
    }

    #[test]
    fn read_session_returns_none_when_file_absent() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.json");
        assert!(read_session(&path).unwrap().is_none());
    }
}
