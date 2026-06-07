use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub(crate) enum MiniboxInput {
    /// List all containers.
    Ps,

    /// Pull an image from Docker Hub.
    Pull {
        image: String,
        #[serde(default = "default_tag")]
        tag: String,
        #[serde(default)]
        platform: Option<String>,
    },

    /// Run a container.
    Run {
        image: String,
        #[serde(default = "default_tag")]
        tag: String,
        #[serde(default)]
        command: Vec<String>,
        #[serde(default)]
        memory: Option<u64>,
        #[serde(default)]
        cpu_weight: Option<u64>,
        #[serde(default = "default_network")]
        network: String,
        #[serde(default)]
        privileged: bool,
        #[serde(default)]
        volumes: Vec<String>,
        #[serde(default)]
        env: Vec<String>,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        platform: Option<String>,
        #[serde(default)]
        rm: bool,
    },

    /// Stop a running container.
    Stop { id: String },

    /// Pause a running container.
    Pause { id: String },

    /// Resume a paused container.
    Resume { id: String },

    /// Remove a stopped container.
    Rm {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        all: bool,
    },

    /// Execute a command in a running container.
    Exec {
        container_id: String,
        cmd: Vec<String>,
    },

    /// Fetch log output from a container.
    Logs {
        id: String,
        #[serde(default)]
        follow: bool,
    },

    /// Run a script in a sandboxed container.
    Sandbox {
        script: String,
        #[serde(default = "default_sandbox_image")]
        image: String,
        #[serde(default = "default_tag")]
        tag: String,
        #[serde(default = "default_memory_mb")]
        memory_mb: u64,
        #[serde(default = "default_timeout")]
        timeout: u64,
        #[serde(default)]
        volumes: Vec<String>,
        #[serde(default)]
        network: bool,
    },

    /// Remove unused images.
    Prune {
        #[serde(default)]
        dry_run: bool,
    },

    /// Remove a specific image by reference (e.g. alpine:latest).
    Rmi { image_ref: String },

    /// Save, restore, or list container snapshots.
    ///
    /// Maps to `mbx snapshot save|restore|list`.
    Snapshot {
        sub_action: SnapshotAction,
        /// Container ID or name.
        container_id: String,
        /// Snapshot name (required for save/restore, ignored for list).
        #[serde(default)]
        name: Option<String>,
    },
}

/// Sub-action for the `snapshot` command.
#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotAction {
    Save,
    Restore,
    List,
}

pub(crate) fn default_tag() -> String {
    "latest".into()
}

pub(crate) fn default_network() -> String {
    "none".into()
}

pub(crate) fn default_sandbox_image() -> String {
    "minibox-sandbox".into()
}

pub(crate) fn default_memory_mb() -> u64 {
    512
}

pub(crate) fn default_timeout() -> u64 {
    60
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn default_tag_returns_latest() {
        assert_eq!(default_tag(), "latest");
    }

    #[test]
    fn default_network_returns_none() {
        assert_eq!(default_network(), "none");
    }

    #[test]
    fn default_sandbox_image_returns_minibox_sandbox() {
        assert_eq!(default_sandbox_image(), "minibox-sandbox");
    }

    #[test]
    fn default_memory_mb_returns_512() {
        assert_eq!(default_memory_mb(), 512);
    }

    #[test]
    fn default_timeout_returns_60() {
        assert_eq!(default_timeout(), 60);
    }

    fn parse_minibox_input(v: serde_json::Value) -> MiniboxInput {
        serde_json::from_value(v).unwrap()
    }

    #[test]
    fn parse_minibox_input_run_action_minimal() {
        let action = parse_minibox_input(json!({ "action": "run", "image": "alpine" }));
        match action {
            MiniboxInput::Run { image, tag, .. } => {
                assert_eq!(image, "alpine");
                assert_eq!(tag, "latest");
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn parse_minibox_input_stop_action() {
        let action = parse_minibox_input(json!({ "action": "stop", "id": "abc123" }));
        assert!(matches!(action, MiniboxInput::Stop { id } if id == "abc123"));
    }

    #[test]
    fn parse_minibox_input_rm_all() {
        let action = parse_minibox_input(json!({ "action": "rm", "all": true }));
        assert!(matches!(action, MiniboxInput::Rm { all: true, .. }));
    }

    #[test]
    fn parse_minibox_input_sandbox_defaults() {
        let action =
            parse_minibox_input(json!({ "action": "sandbox", "script": "/tmp/foo.py" }));
        match action {
            MiniboxInput::Sandbox {
                memory_mb, timeout, ..
            } => {
                assert_eq!(memory_mb, 512);
                assert_eq!(timeout, 60);
            }
            _ => panic!("expected Sandbox"),
        }
    }

    #[test]
    fn parse_minibox_input_ps() {
        let action = parse_minibox_input(json!({ "action": "ps" }));
        assert!(matches!(action, MiniboxInput::Ps));
    }

    #[test]
    fn parse_minibox_input_snapshot_save() {
        let action = parse_minibox_input(json!({
            "action": "snapshot",
            "sub_action": "save",
            "container_id": "abc123",
            "name": "snap1"
        }));
        match action {
            MiniboxInput::Snapshot {
                sub_action: SnapshotAction::Save,
                container_id,
                name,
            } => {
                assert_eq!(container_id, "abc123");
                assert_eq!(name, Some("snap1".to_string()));
            }
            _ => panic!("expected Snapshot/Save"),
        }
    }

    #[test]
    fn parse_minibox_input_snapshot_list() {
        let action = parse_minibox_input(json!({
            "action": "snapshot",
            "sub_action": "list",
            "container_id": "abc123"
        }));
        assert!(matches!(
            action,
            MiniboxInput::Snapshot {
                sub_action: SnapshotAction::List,
                ..
            }
        ));
    }

    #[test]
    fn parse_minibox_input_snapshot_restore() {
        let action = parse_minibox_input(json!({
            "action": "snapshot",
            "sub_action": "restore",
            "container_id": "abc123",
            "name": "snap1"
        }));
        assert!(matches!(
            action,
            MiniboxInput::Snapshot {
                sub_action: SnapshotAction::Restore,
                ..
            }
        ));
    }
}
