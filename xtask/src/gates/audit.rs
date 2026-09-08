//! audit — cargo-deny advisory gate.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo deny check advisories`
pub fn audit(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo deny check advisories")
        .run()
        .context("cargo deny advisories failed")
}
