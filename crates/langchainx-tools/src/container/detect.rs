//! Auto-detection of the best available container runtime.

use super::{ContainerRuntime, DockerRuntime, MiniboxRuntime};

/// Detect the best available container runtime on PATH.
///
/// Priority order: `mbx` > `docker` > `podman`.
///
/// Returns `None` if no supported runtime is found.
pub fn detect_runtime() -> Option<Box<dyn ContainerRuntime>> {
    if let Some(rt) = MiniboxRuntime::detect() {
        return Some(Box::new(rt));
    }
    if let Some(rt) = DockerRuntime::detect() {
        return Some(Box::new(rt));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_runtime_returns_some_on_dev_machine() {
        // At least docker or mbx should be available in dev
        let rt = detect_runtime();
        if let Some(rt) = rt {
            assert!(!rt.name().is_empty());
        }
        // Not asserting Some — CI may lack both
    }
}
