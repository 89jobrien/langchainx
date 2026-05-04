//! gates — quality gates that mirror CI steps exactly.
//!
//! Each submodule owns one gate. `ci` composes the full pipeline in the same
//! order as `.github/workflows/ci.yml`. `pre_commit` is the fast local subset.
//!
//! | Gate         | Command                                      |
//! |--------------|----------------------------------------------|
//! | fmt_check    | cargo fmt --all -- --check                   |
//! | clippy       | cargo clippy --all-features -- -D warnings   |
//! | build        | cargo build --release --all-features         |
//! | test         | cargo test --release --all-features          |
//! | ci           | fmt_check → clippy → build → test            |
//! | pre_commit   | fmt_check → clippy                           |

mod build;
mod clippy;
mod fmt;
mod test;

pub use build::build;
pub use clippy::clippy;
pub use fmt::fmt_check;
pub use test::test;

use anyhow::Result;
use xshell::Shell;

/// Full CI pipeline: fmt_check → clippy → build → test.
///
/// Identical to what `.github/workflows/ci.yml` runs. Use this locally before
/// pushing to guarantee CI will pass.
pub fn ci(sh: &Shell) -> Result<()> {
    fmt_check(sh)?;
    clippy(sh)?;
    build(sh)?;
    test(sh)?;
    eprintln!("ci passed");
    Ok(())
}

/// Local pre-commit gate: fmt_check → clippy.
///
/// Fast subset — no build or test. Run before every commit.
pub fn pre_commit(sh: &Shell) -> Result<()> {
    fmt_check(sh)?;
    clippy(sh)?;
    eprintln!("pre-commit checks passed");
    Ok(())
}
