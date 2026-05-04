//! test — cargo test gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo test --release --all-features`
pub fn test(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo test --release --all-features")
        .run()
        .context("tests failed")
}
