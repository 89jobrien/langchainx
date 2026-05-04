//! fmt — cargo fmt check gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo fmt --all -- --check`
pub fn fmt_check(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo fmt --all -- --check")
        .run()
        .context("fmt check failed")
}
