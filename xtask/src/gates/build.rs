//! build — cargo build gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo build --release --all-features`
pub fn build(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo build --release --all-features")
        .run()
        .context("build failed")
}
