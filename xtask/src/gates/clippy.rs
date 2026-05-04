//! clippy — cargo clippy gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo clippy --all-features -- -D warnings`
pub fn clippy(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo clippy --all-features -- -D warnings")
        .run()
        .context("clippy failed")
}
