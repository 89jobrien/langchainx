//! gates — quality gates that mirror CI steps exactly.
//!
//! Each submodule owns one leaf gate. `gate` provides the composition
//! primitives. `ci` and `pre_commit` are named pipelines built from gates.
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
pub mod gate;

pub use build::build;
pub use clippy::clippy;
pub use fmt::fmt_check;
pub use test::test;
pub use gate::{Gate, run_gates};

use anyhow::Result;
use xshell::Shell;

/// Full CI pipeline: fmt_check → clippy → build → test.
///
/// Identical to what `.github/workflows/ci.yml` runs. Use this locally before
/// pushing to guarantee CI will pass.
pub fn ci(sh: &Shell) -> Result<()> {
    run_gates(sh, &[
        Gate::new("fmt-check", fmt_check),
        Gate::new("clippy",    clippy),
        Gate::new("build",     build),
        Gate::new("test",      test),
    ])?;
    eprintln!("ci passed");
    Ok(())
}

/// Local pre-commit gate: fmt_check → clippy.
///
/// Fast subset — no build or test. Run before every commit.
pub fn pre_commit(sh: &Shell) -> Result<()> {
    run_gates(sh, &[
        Gate::new("fmt-check", fmt_check),
        Gate::new("clippy",    clippy),
    ])?;
    eprintln!("pre-commit checks passed");
    Ok(())
}
