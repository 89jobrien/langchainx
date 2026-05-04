use std::path::Path;

use tempfile::TempDir;

/// A temporary directory scoped to a single test.
///
/// Automatically deleted on drop. Use with `CommandExecutor::with_working_dir`
/// to give agents an isolated filesystem sandbox.
///
/// ```rust
/// use langchainx::test_utils::TempWorkspace;
///
/// let ws = TempWorkspace::new();
/// ws.write_file("hello.txt", "world");
/// assert_eq!(ws.read_file("hello.txt"), "world");
/// // deleted when `ws` drops
/// ```
pub struct TempWorkspace {
    dir: TempDir,
}

impl TempWorkspace {
    pub fn new() -> Self {
        Self {
            dir: TempDir::new().expect("failed to create temp workspace"),
        }
    }

    /// Absolute path to the workspace root.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Write `content` to `name` inside the workspace.
    pub fn write_file(&self, name: &str, content: &str) {
        std::fs::write(self.dir.path().join(name), content)
            .expect("failed to write file in TempWorkspace");
    }

    /// Read the contents of `name` from the workspace.
    pub fn read_file(&self, name: &str) -> String {
        std::fs::read_to_string(self.dir.path().join(name))
            .expect("failed to read file in TempWorkspace")
    }

    /// Returns true if `name` exists in the workspace.
    pub fn exists(&self, name: &str) -> bool {
        self.dir.path().join(name).exists()
    }
}

impl Default for TempWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_file() {
        let ws = TempWorkspace::new();
        ws.write_file("foo.txt", "bar");
        assert_eq!(ws.read_file("foo.txt"), "bar");
    }

    #[test]
    fn exists_returns_true_after_write() {
        let ws = TempWorkspace::new();
        assert!(!ws.exists("missing.txt"));
        ws.write_file("missing.txt", "");
        assert!(ws.exists("missing.txt"));
    }

    #[test]
    fn path_is_a_directory() {
        let ws = TempWorkspace::new();
        assert!(ws.path().is_dir());
    }

    #[test]
    fn dropped_workspace_is_deleted() {
        let path = {
            let ws = TempWorkspace::new();
            let p = ws.path().to_path_buf();
            assert!(p.exists());
            p
        };
        assert!(!path.exists());
    }
}
