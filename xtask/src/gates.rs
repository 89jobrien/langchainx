//! gates — quality gates that mirror CI steps.
//!
//! Each function corresponds to one CI step. `pre_commit` runs the local
//! subset (fast, no network) that should pass before every commit.

use anyhow::{Context, Result};
use xshell::{Shell, cmd};

/// `cargo fmt --all -- --check`
pub fn fmt_check(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo fmt --all -- --check")
        .run()
        .context("fmt check failed")
}

/// `cargo clippy --all-features -- -D warnings`
pub fn clippy(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo clippy --all-features -- -D warnings")
        .run()
        .context("clippy failed")
}

/// `cargo build --release --all-features`
pub fn build(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo build --release --all-features")
        .run()
        .context("build failed")
}

/// `cargo test --release --all-features`
pub fn test(sh: &Shell) -> Result<()> {
    cmd!(sh, "cargo test --release --all-features")
        .run()
        .context("tests failed")
}

/// Local pre-commit gate: fmt-check + clippy.
///
/// Matches the first two steps of CI. Run before committing.
pub fn pre_commit(sh: &Shell) -> Result<()> {
    fmt_check(sh)?;
    clippy(sh)?;
    eprintln!("pre-commit checks passed");
    Ok(())
}
