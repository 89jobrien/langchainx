//! Agent checkpoint — resumable agents with per-step checkpoint files.
//!
//! Checkpoint files are written to `.ctx/GODMODE.agent.<agent_id>.checkpoint.json`
//! relative to the workspace root. An agent that is interrupted can resume from its
//! last recorded step rather than starting over.
//!
//! # Lifecycle
//!
//! 1. On start, call [`checkpoint_read`] to detect an existing (non-expired) checkpoint.
//! 2. After each step completes, call [`checkpoint_write`] with the new [`CheckpointStep`].
//! 3. On clean finish, call [`checkpoint_clear`] to remove the file.
//!
//! Checkpoints older than [`DEFAULT_MAX_AGE`] (24 hours) are treated as expired and
//! ignored by [`checkpoint_read`].

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Default maximum age before a checkpoint is considered stale.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24);

/// The directory (relative to workspace root) where checkpoint files are written.
const CHECKPOINT_DIR: &str = ".ctx";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Discrete steps in an agent's execution pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointStep {
    /// Agent is reading context / inputs.
    Reading,
    /// Tests have been written but implementation is not yet done.
    TestWritten,
    /// Implementation is complete; tests are passing.
    ImplDone,
    /// Changes have been committed to the repository.
    Committed,
}

/// A persisted snapshot of an agent's progress.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCheckpoint {
    /// Opaque identifier for the agent (e.g. issue number or UUID).
    pub agent_id: String,
    /// The last successfully completed step.
    pub step: CheckpointStep,
    /// Unix timestamp (seconds) when this checkpoint was written.
    pub timestamp: u64,
    /// Arbitrary key-value metadata (e.g. file paths, hashes, notes).
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// File path helper
// ---------------------------------------------------------------------------

/// Returns the checkpoint file path for the given `agent_id`.
///
/// The file lives at `<root>/.ctx/GODMODE.agent.<agent_id>.checkpoint.json`.
pub fn checkpoint_path(root: &Path, agent_id: &str) -> PathBuf {
    root.join(CHECKPOINT_DIR)
        .join(format!("GODMODE.agent.{agent_id}.checkpoint.json"))
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Write (or overwrite) the checkpoint for `agent_id` with `step` and optional `metadata`.
///
/// Creates `.ctx/` if it does not yet exist. The timestamp is set to the current
/// system time automatically.
///
/// # Errors
///
/// Returns an error if the directory cannot be created or the file cannot be written.
pub fn checkpoint_write(
    root: &Path,
    agent_id: &str,
    step: CheckpointStep,
    metadata: HashMap<String, String>,
) -> Result<()> {
    let dir = root.join(CHECKPOINT_DIR);
    fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create checkpoint dir {dir:?}"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let checkpoint = AgentCheckpoint {
        agent_id: agent_id.to_owned(),
        step,
        timestamp,
        metadata,
    };

    let path = checkpoint_path(root, agent_id);
    let json = serde_json::to_string_pretty(&checkpoint)
        .context("failed to serialise checkpoint")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write checkpoint to {path:?}"))
}

/// Read the checkpoint for `agent_id`, returning `None` if none exists or it is expired.
///
/// A checkpoint is expired when [`is_expired`] returns `true` using [`DEFAULT_MAX_AGE`].
///
/// # Errors
///
/// Returns an error only when the checkpoint file exists but cannot be parsed.
pub fn checkpoint_read(root: &Path, agent_id: &str) -> Result<Option<AgentCheckpoint>> {
    let path = checkpoint_path(root, agent_id);
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed to read checkpoint from {path:?}"))?;
    let checkpoint: AgentCheckpoint =
        serde_json::from_str(&raw).with_context(|| format!("malformed checkpoint at {path:?}"))?;

    if is_expired(&checkpoint, DEFAULT_MAX_AGE) {
        return Ok(None);
    }

    Ok(Some(checkpoint))
}

/// Remove the checkpoint file for `agent_id`, if it exists.
///
/// Silently succeeds if the file does not exist.
///
/// # Errors
///
/// Returns an error if the file exists but cannot be removed.
pub fn checkpoint_clear(root: &Path, agent_id: &str) -> Result<()> {
    let path = checkpoint_path(root, agent_id);
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove checkpoint {path:?}"))?;
    }
    Ok(())
}

/// Returns `true` when the checkpoint's age exceeds `max_age`.
pub fn is_expired(checkpoint: &AgentCheckpoint, max_age: Duration) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(checkpoint.timestamp) > max_age.as_secs()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn write_then_read_returns_same_data() {
        let dir = tmp();
        let root = dir.path();

        let mut meta = HashMap::new();
        meta.insert("branch".to_owned(), "issue/63".to_owned());

        checkpoint_write(root, "agent-42", CheckpointStep::TestWritten, meta.clone())
            .expect("write");

        let got = checkpoint_read(root, "agent-42")
            .expect("read")
            .expect("checkpoint present");

        assert_eq!(got.agent_id, "agent-42");
        assert_eq!(got.step, CheckpointStep::TestWritten);
        assert_eq!(got.metadata, meta);
    }

    #[test]
    fn clear_removes_file() {
        let dir = tmp();
        let root = dir.path();

        checkpoint_write(root, "agent-7", CheckpointStep::Reading, HashMap::new())
            .expect("write");
        assert!(checkpoint_path(root, "agent-7").exists());

        checkpoint_clear(root, "agent-7").expect("clear");
        assert!(!checkpoint_path(root, "agent-7").exists());
    }

    #[test]
    fn clear_is_idempotent_when_missing() {
        let dir = tmp();
        checkpoint_clear(dir.path(), "nonexistent").expect("clear on missing file should succeed");
    }

    #[test]
    fn read_nonexistent_returns_none() {
        let dir = tmp();
        let result = checkpoint_read(dir.path(), "no-such-agent").expect("read");
        assert!(result.is_none());
    }

    #[test]
    fn expired_checkpoint_detected() {
        let checkpoint = AgentCheckpoint {
            agent_id: "agent-1".to_owned(),
            step: CheckpointStep::ImplDone,
            // timestamp far in the past
            timestamp: 0,
            metadata: HashMap::new(),
        };

        assert!(is_expired(&checkpoint, DEFAULT_MAX_AGE));
    }

    #[test]
    fn fresh_checkpoint_not_expired() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let checkpoint = AgentCheckpoint {
            agent_id: "agent-2".to_owned(),
            step: CheckpointStep::Committed,
            timestamp: now,
            metadata: HashMap::new(),
        };

        assert!(!is_expired(&checkpoint, DEFAULT_MAX_AGE));
    }

    #[test]
    fn expired_checkpoint_read_returns_none() {
        let dir = tmp();
        let root = dir.path();

        // Write a checkpoint then manually backdate its timestamp.
        checkpoint_write(root, "old-agent", CheckpointStep::Reading, HashMap::new())
            .expect("write");

        let path = checkpoint_path(root, "old-agent");
        let raw = fs::read_to_string(&path).unwrap();
        let mut cp: AgentCheckpoint = serde_json::from_str(&raw).unwrap();
        cp.timestamp = 0; // force expiry
        fs::write(&path, serde_json::to_string_pretty(&cp).unwrap()).unwrap();

        let result = checkpoint_read(root, "old-agent").expect("read");
        assert!(result.is_none(), "expired checkpoint should return None");
    }
}
