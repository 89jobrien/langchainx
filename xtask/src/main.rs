//! xtask — workspace dev-tool binary.
//!
//! Each module has one clear responsibility. Add new tasks by creating a new
//! module and wiring it into the `match` below; do NOT grow existing modules
//! beyond their stated scope.
//!
//! | Module   | Responsibility                                         |
//! |----------|--------------------------------------------------------|
//! | `gates`  | Quality gates: fmt-check, clippy, build, test          |
//! | `bump`   | Semver version bump in workspace Cargo.toml            |

use anyhow::{Result, bail};
use std::{env, path::Path};
use xshell::Shell;

mod bump;
mod gates;

fn main() -> Result<()> {
    let task = env::args().nth(1);

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    env::set_current_dir(root)?;

    let sh = Shell::new()?;
    sh.change_dir(root);

    match task.as_deref() {
        Some("fmt-check") => gates::fmt_check(&sh),
        Some("clippy") => gates::clippy(&sh),
        Some("build") => gates::build(&sh),
        Some("test") => gates::test(&sh),
        Some("pre-commit") => gates::pre_commit(&sh),
        Some("bump") => {
            let level = env::args().nth(2).unwrap_or_else(|| "patch".to_string());
            bump::bump(root, &level)
        }
        Some(other) => bail!("unknown task: {other}"),
        None => {
            eprintln!("usage: cargo xtask <task>");
            eprintln!();
            eprintln!("tasks:");
            eprintln!("  fmt-check    cargo fmt --check");
            eprintln!("  clippy       cargo clippy --all-features");
            eprintln!("  build        cargo build --release --all-features");
            eprintln!("  test         cargo test --release --all-features");
            eprintln!("  pre-commit   fmt-check + clippy (local CI gate)");
            eprintln!("  bump         bump workspace version (patch|minor|major)");
            Ok(())
        }
    }
}
